//! Route modules and top-level router construction.

pub mod addresses;
pub mod answer_bank;
pub mod archetypes;
pub mod audit;
pub mod bullets;
pub mod campuses;
pub mod certifications;
pub mod contacts;
pub mod credentials;
pub mod derivations;
pub mod domains;
pub mod export;
pub mod health;
pub mod industries;
pub mod integrity;
pub mod job_descriptions;
pub mod notes;
pub mod organizations;
pub mod perspectives;
pub mod profile;
pub mod resumes;
pub mod review;
pub mod role_types;
pub mod skills;
pub mod sources;
pub mod summaries;
pub mod supporting;
pub mod templates;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::Router;
use serde_json::json;

use crate::state::SharedState;

/// Build the full API router. All routes are nested under `/api`.
pub fn api_router() -> Router<SharedState> {
    let api = Router::new()
        .route("/health", axum::routing::get(health::health))
        .merge(sources::router())
        .merge(bullets::router())
        .merge(perspectives::router())
        .merge(organizations::router())
        .merge(skills::router())
        .merge(supporting::router())
        .merge(profile::router())
        .merge(notes::router())
        .merge(resumes::router())
        .merge(summaries::router())
        .merge(contacts::router())
        .merge(job_descriptions::router())
        .merge(addresses::router())
        .merge(domains::router())
        .merge(industries::router())
        .merge(role_types::router())
        .merge(certifications::router())
        .merge(archetypes::router())
        .merge(campuses::router())
        .merge(credentials::router())
        .merge(answer_bank::router())
        .merge(derivations::router())
        .merge(audit::router())
        .merge(review::router())
        .merge(integrity::router())
        .merge(export::router())
        .merge(templates::router());

    Router::new()
        .nest("/api", api)
        .fallback(fallback)
}

async fn fallback(
    method: axum::http::Method,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "code": "NOT_FOUND",
                "message": format!("Route not found: {method} {uri}")
            }
        })),
    )
}
