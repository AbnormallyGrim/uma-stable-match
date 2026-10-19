//! Client errors.

use reqwest::StatusCode;
use serde::Deserialize;

/// Error body returned by the API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApiErrorBody {
    /// Short error message.
    pub error: String,
    /// HTTP status code.
    pub status: u16,
    /// Extra detail, if any.
    #[serde(default)]
    pub details: Option<String>,
}

/// Errors from [`Client`](crate::Client).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The settings are invalid.
    #[error("invalid client config: {0}")]
    Config(String),
    /// The request parameters are invalid.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// The API returned an error status.
    #[error("uma.moe returned {status}{}", body.as_ref().map(|b| format!(": {}", b.error)).unwrap_or_default())]
    Status {
        /// HTTP status.
        status: StatusCode,
        /// Parsed error body, if the API sent one.
        body: Option<ApiErrorBody>,
    },
    /// The request could not be sent or the response could not be read.
    #[error("request to uma.moe failed: {0}")]
    Transport(#[source] reqwest::Error),
    /// The response body did not match the expected shape.
    #[error("unexpected response from uma.moe: {0}")]
    Decode(#[source] serde_json::Error),
}

impl Error {
    /// HTTP status, if the API returned one.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
}
