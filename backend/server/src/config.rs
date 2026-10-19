//! Server settings.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use uma_stable_match_umamoe_client::ApiKey;

use crate::pace::DEFAULT_PACE_TTL;
use crate::scanner::ScanConfig;

pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

pub const DEFAULT_PORT: u16 = 8080;

// TODO(#51): adjust the default once uma.moe agrees how often we may scan.
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
    pub trainer_cache: Duration,
}

/// Invalid or missing settings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// `UMAMOE_API_KEY` is missing or blank.
    #[error("UMAMOE_API_KEY is not set")]
    MissingApiKey,
    #[error("HOST must be an IP address, got {0:?}")]
    InvalidHost(String),
    #[error("PORT must be a port number from 1 to 65535, got {0:?}")]
    InvalidPort(String),
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
        let host = match get("HOST") {
            None => DEFAULT_HOST,
            Some(value) => value.parse().map_err(|_| ConfigError::InvalidHost(value))?,
        };
        let port = match get("PORT") {
            None => DEFAULT_PORT,
            Some(value) => value
                .parse::<u16>()
                .ok()
                .filter(|p| *p > 0)
                .ok_or(ConfigError::InvalidPort(value))?,
        };
        let bind_addr = SocketAddr::new(host, port);

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
        let trainer_cache = Duration::from_secs(
            u64::from(number(
                "TRAINER_CACHE_MINUTES",
                (DEFAULT_PACE_TTL.as_secs() / 60) as u32,
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
            trainer_cache,
        })
    }
}

#[cfg(test)]
mod tests;
