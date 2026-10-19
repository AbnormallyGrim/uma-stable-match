//! JSON error responses.

use std::time::Duration;

use axum::http::header::RETRY_AFTER;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
    pub(crate) retry_after: Option<u64>,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            retry_after: None,
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub(crate) fn rate_limited(wait: Duration) -> Self {
        let seconds = wait.as_secs_f64().ceil().max(1.0) as u64;
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: format!("too many requests; try again in {seconds} seconds"),
            retry_after: Some(seconds),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(json!({ "error": self.message }))).into_response();
        if let Some(seconds) = self.retry_after {
            response.headers_mut().insert(RETRY_AFTER, seconds.into());
        }
        response
    }
}
