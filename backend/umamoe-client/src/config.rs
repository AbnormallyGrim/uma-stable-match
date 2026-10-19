use std::fmt;
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://uma.moe";

#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
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

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub api_key: ApiKey,
    pub timeout: Duration,
    pub user_agent: String,
}

impl ClientConfig {
    pub fn new(api_key: ApiKey) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key,
            timeout: Duration::from_secs(15),
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
