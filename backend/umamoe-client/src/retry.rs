//! Retry rules and backoff delays.

use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::StatusCode;

use crate::config::RetryConfig;

/// Whether a response status is worth retrying.
pub(crate) fn is_retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
}

/// Whether a transport error is worth retrying.
pub(crate) fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

/// `Retry-After` in seconds, if present. HTTP dates are ignored.
pub(crate) fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

/// Delay before retry number `retry` (0-based), with `jitter` in `[0, 1)`.
pub(crate) fn delay(
    config: &RetryConfig,
    retry: u32,
    retry_after: Option<Duration>,
    jitter: f64,
) -> Duration {
    retry_after.unwrap_or_else(|| {
        // Half fixed, half random, so clients don't retry in lockstep.
        let exp = config
            .base_delay
            .saturating_mul(2u32.saturating_pow(retry))
            .min(config.max_delay);
        exp / 2 + exp.mul_f64(jitter.clamp(0.0, 1.0) / 2.0)
    })
}

#[cfg(test)]
mod tests;
