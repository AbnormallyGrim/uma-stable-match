use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub status: u16,
    #[serde(default)]
    pub details: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid client config: {0}")]
    Config(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("uma.moe returned {status}{}", body.as_ref().map(|b| format!(": {}", b.error)).unwrap_or_default())]
    Status {
        status: StatusCode,
        body: Option<ApiErrorBody>,
    },
    #[error("request to uma.moe failed: {0}")]
    Transport(#[source] reqwest::Error),
    #[error("unexpected response from uma.moe: {0}")]
    Decode(#[source] serde_json::Error),
}

impl Error {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
}
