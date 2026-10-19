//! Server settings.

use std::net::SocketAddr;

use uma_stable_match_umamoe_client::ApiKey;

/// Address the server listens on when `BIND_ADDR` isn't set.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// Server settings. `Debug` output redacts the API key.
#[derive(Debug, Clone)]
pub struct Config {
    /// uma.moe API key, from `UMAMOE_API_KEY`.
    pub api_key: ApiKey,
    /// uma.moe base URL override, from `UMAMOE_BASE_URL`.
    pub base_url: Option<String>,
    /// Listen address, from `BIND_ADDR`.
    pub bind_addr: SocketAddr,
}

/// Invalid or missing settings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// `UMAMOE_API_KEY` is missing or blank.
    #[error("UMAMOE_API_KEY is not set")]
    MissingApiKey,
    /// `BIND_ADDR` isn't a socket address.
    #[error("BIND_ADDR is not a valid address: {0}")]
    InvalidBindAddr(String),
}

impl Config {
    /// Reads settings from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads settings through `lookup`. Blank values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };

        let api_key = get("UMAMOE_API_KEY")
            .map(ApiKey::new)
            .ok_or(ConfigError::MissingApiKey)?;
        let bind_addr = get("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind_addr
            .parse()
            .map_err(|_| ConfigError::InvalidBindAddr(bind_addr))?;

        Ok(Self {
            api_key,
            base_url: get("UMAMOE_BASE_URL"),
            bind_addr,
        })
    }
}

#[cfg(test)]
mod tests;
