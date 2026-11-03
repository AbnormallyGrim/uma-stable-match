//! Errors from Discord calls.

use std::time::Duration;

use reqwest::StatusCode;

/// Why a Discord call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request couldn't be sent or its response couldn't be read.
    #[error("request to Discord failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Discord answered with an error.
    #[error("Discord answered {status}: {message}")]
    Api {
        /// HTTP status.
        status: StatusCode,
        /// Discord's JSON error code, if given.
        code: Option<u32>,
        /// Discord's error message, or the status's reason.
        message: String,
    },
    /// The rate limit would make this request wait longer than allowed, or it was retried too
    /// often.
    #[error("Discord is rate limiting this request; retry in {} seconds", .0.as_secs_f64().ceil())]
    RateLimited(Duration),
    /// The response wasn't the JSON expected.
    #[error("unexpected response from Discord: {0}")]
    Decode(#[source] serde_json::Error),
}

impl Error {
    /// The HTTP status Discord answered with, if it answered with an error.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api { status, .. } => Some(*status),
            _ => None,
        }
    }
}
