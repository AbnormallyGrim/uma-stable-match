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

/// Settings for [`Client`](crate::Client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
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

#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API base URL, without a trailing path.
    pub base_url: String,
    /// API key sent as `X-API-Key`.
    pub api_key: ApiKey,
    /// Timeout for each request.
    pub timeout: Duration,
    /// Minimum gap between requests.
    pub min_interval: Duration,
    pub retry: RetryConfig,
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
