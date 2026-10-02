//! Response envelopes, matching the TypeScript API's JSON shapes exactly:
//! `{data}`, `{data, pagination}`, 201 `{data}`, 204 empty.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use forge_core::Pagination;
use serde::Serialize;

/// `{ "data": T }`
#[derive(Serialize)]
pub struct ApiData<T> {
    pub data: T,
}

/// `{ "data": [T], "pagination": { total, offset, limit } }`
#[derive(Serialize)]
pub struct ApiList<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

/// 201 with `{ "data": T }`.
pub struct Created<T>(pub T);

impl<T: Serialize> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(ApiData { data: self.0 })).into_response()
    }
}

/// 204 with an empty body.
pub struct NoContent;

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}
