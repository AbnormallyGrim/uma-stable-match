//! Client settings.

use std::fmt;
use std::time::Duration;

/// Default API base URL.
pub const DEFAULT_BASE_URL: &str = "https://uma.moe";

/// An uma.moe API key. `Debug` output is redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Wraps a key.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

/// Retry settings for failed requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    /// Retries after the first attempt.
    pub max_retries: u32,
    /// Delay before the first retry; doubles on each retry.
    pub base_delay: Duration,
    /// Longest delay between attempts, including `Retry-After`.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackoffConfig {
    pub max_interval: Duration,
    pub recovery_step: f64,
    pub recovery_every: Duration,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            max_interval: Duration::from_secs(30),
            recovery_step: 0.05,
            recovery_every: Duration::from_secs(10),
        }
    }
}

/// Settings for [`Client`](crate::Client).
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API base URL, without a trailing path.
    pub base_url: String,
    /// API key sent as `X-API-Key`.
    pub api_key: ApiKey,
    /// Timeout for each attempt.
    pub timeout: Duration,
    /// Minimum gap between requests, including retries.
    pub min_interval: Duration,
    /// Retry settings.
    pub retry: RetryConfig,
    pub backoff: BackoffConfig,
    /// `User-Agent` header value.
    pub user_agent: String,
}

impl ClientConfig {
    /// Settings with defaults for everything but the key.
    pub fn new(api_key: ApiKey) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key,
            timeout: Duration::from_secs(15),
            min_interval: Duration::from_millis(500),
            retry: RetryConfig::default(),
            backoff: BackoffConfig::default(),
            user_agent: concat!(
                "uma-stable-match/",
                env!("CARGO_PKG_VERSION"),
                " (+",
                env!("CARGO_PKG_REPOSITORY"),
                ")"
            )
            .to_owned(),
        }
    }
}

#[cfg(test)]
mod tests;
