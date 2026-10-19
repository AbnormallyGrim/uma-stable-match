use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::StatusCode;

use crate::config::RetryConfig;

pub(crate) fn is_retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
}

pub(crate) fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

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

pub(crate) fn delay(
    config: &RetryConfig,
    retry: u32,
    retry_after: Option<Duration>,
    jitter: f64,
) -> Duration {
    let delay = retry_after.unwrap_or_else(|| {
        let exp = config
            .base_delay
            .saturating_mul(2u32.saturating_pow(retry))
            .min(config.max_delay);
        exp / 2 + exp.mul_f64(jitter.clamp(0.0, 1.0) / 2.0)
    });
    delay.min(config.max_delay)
}

#[cfg(test)]
mod tests;
