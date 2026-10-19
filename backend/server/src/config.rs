//! Server settings.

use std::net::SocketAddr;
use std::time::Duration;

use uma_stable_match_umamoe_client::ApiKey;

use crate::scanner::ScanConfig;

/// Address the server listens on when `BIND_ADDR` isn't set.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// Minutes between scans when `SCAN_INTERVAL_MINUTES` isn't set.
pub const DEFAULT_SCAN_INTERVAL_MINUTES: u32 = 180;

/// Server settings. `Debug` output redacts the API key.
#[derive(Debug, Clone)]
pub struct Config {
    /// uma.moe API key, from `UMAMOE_API_KEY`.
    pub api_key: ApiKey,
    /// uma.moe base URL override, from `UMAMOE_BASE_URL`.
    pub base_url: Option<String>,
    /// Listen address, from `BIND_ADDR`.
    pub bind_addr: SocketAddr,
    /// Scan settings, from `SCAN_RANK_FROM`, `SCAN_RANK_TO` and `SCAN_MAX_DETAILS`.
    pub scan: ScanConfig,
    /// Time between scans, from `SCAN_INTERVAL_MINUTES`.
    pub scan_interval: Duration,
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
    /// A numeric setting isn't a positive whole number.
    #[error("{name} must be a positive whole number, got {value:?}")]
    InvalidNumber {
        /// Variable name.
        name: &'static str,
        /// Value that was set.
        value: String,
    },
    /// The rank window is inverted.
    #[error("SCAN_RANK_FROM ({from}) must not be above SCAN_RANK_TO ({to})")]
    InvalidRankWindow {
        /// `SCAN_RANK_FROM`.
        from: u32,
        /// `SCAN_RANK_TO`.
        to: u32,
    },
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

        let number = |name: &'static str, default: u32| match get(name) {
            None => Ok(default),
            Some(value) => value
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or(ConfigError::InvalidNumber { name, value }),
        };
        let defaults = ScanConfig::default();
        let scan = ScanConfig {
            rank_from: number("SCAN_RANK_FROM", defaults.rank_from)?,
            rank_to: number("SCAN_RANK_TO", defaults.rank_to)?,
            max_details: number("SCAN_MAX_DETAILS", defaults.max_details)?,
            ..defaults
        };
        let scan_interval = Duration::from_secs(
            u64::from(number(
                "SCAN_INTERVAL_MINUTES",
                DEFAULT_SCAN_INTERVAL_MINUTES,
            )?) * 60,
        );
        if scan.rank_from > scan.rank_to {
            return Err(ConfigError::InvalidRankWindow {
                from: scan.rank_from,
                to: scan.rank_to,
            });
        }

        Ok(Self {
            api_key,
            base_url: get("UMAMOE_BASE_URL"),
            bind_addr,
            scan,
            scan_interval,
        })
    }
}

#[cfg(test)]
mod tests;
