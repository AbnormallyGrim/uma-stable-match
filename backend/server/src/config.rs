use std::net::SocketAddr;

use uma_stable_match_umamoe_client::ApiKey;

pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: ApiKey,
    pub base_url: Option<String>,
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("UMAMOE_API_KEY is not set")]
    MissingApiKey,
    #[error("BIND_ADDR is not a valid address: {0}")]
    InvalidBindAddr(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

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
