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
pub mod extension;
pub mod health;
pub mod industries;
pub mod integrity;
pub mod job_descriptions;
pub mod notes;
pub mod organizations;
pub mod pdf;
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

use axum::extract::OriginalUri;
use axum::http::{Method, StatusCode, Uri};
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
        .merge(extension::router())
        .merge(templates::router())
        // Must come after every route is added: it covers the routes that exist when it is set.
        .method_not_allowed_fallback(method_not_allowed);

    Router::new().nest("/api", api).fallback(fallback)
}

/// The TS server's answer for a route it doesn't have: 404, `Route not found: <METHOD> <path>`.
fn route_not_found(method: &Method, uri: &Uri) -> impl IntoResponse {
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

/// A path nothing serves.
async fn fallback(method: Method, uri: Uri) -> impl IntoResponse {
    route_not_found(&method, &uri)
}

/// A path the API serves, hit with a method it doesn't (a `PATCH` on a route that has only `GET`
/// and `DELETE`). axum answers that with a bare 405 by default; the TS server answers it exactly
/// like an unknown path, because Hono sends a method mismatch to `notFound`
/// (`packages/core/src/routes/server.ts`). Rust does the same, so the two runtimes agree and the
/// browser dispatcher's existing 404-to-501 rewrite covers "this endpoint is not ported yet" for
/// methods as well as paths (resu-mx/forge#124). `OriginalUri`, because the nested router has
/// stripped the `/api` prefix from `Uri` and the message must show the full path.
async fn method_not_allowed(method: Method, OriginalUri(uri): OriginalUri) -> impl IntoResponse {
    route_not_found(&method, &uri)
}
