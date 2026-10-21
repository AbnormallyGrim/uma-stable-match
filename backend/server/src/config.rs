//! Server settings.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use uma_stable_match_umamoe_client::ApiKey;

use crate::pace::DEFAULT_PACE_TTL;
use crate::recommendations::MatchSettings;
use crate::scanner::ScanConfig;
use crate::shared::{KeySecret, RedisUrl};

/// Address the server listens on when `HOST` isn't set.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Port the server listens on when `PORT` isn't set.
pub const DEFAULT_PORT: u16 = 8080;

/// Recommendation requests per visitor per minute when `RATE_LIMIT_PER_MINUTE` isn't set.
pub const DEFAULT_RATE_LIMIT_PER_MINUTE: u32 = 30;

/// Uncached trainer lookups per visitor per minute when `TRAINER_LOOKUPS_PER_MINUTE` isn't set.
pub const DEFAULT_TRAINER_LOOKUPS_PER_MINUTE: u32 = 10;

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
    /// Listen address, from `HOST` and `PORT`.
    pub bind_addr: SocketAddr,
    /// Scan settings, from `SCAN_RANK_FROM`, `SCAN_RANK_TO` and `SCAN_MAX_DETAILS`.
    pub scan: ScanConfig,
    /// Time between scans, from `SCAN_INTERVAL_MINUTES`.
    pub scan_interval: Option<Duration>,
    /// How often club details are refreshed, from `DETAIL_CACHE_MINUTES`. `None` follows uma.moe's
    /// daily update, or the scan interval with Redis.
    pub detail_cache: Option<Duration>,
    /// Shortlist and fetch settings, from `SHORTLIST_SIZE` and `REQUEST_FETCH_BUDGET`.
    pub matching: MatchSettings,
    /// How long a trainer's pace is reused, from `TRAINER_CACHE_MINUTES`.
    pub trainer_cache: Duration,
    /// Recommendation requests per visitor per minute, from `RATE_LIMIT_PER_MINUTE`.
    pub rate_limit_per_minute: u32,
    /// Uncached trainer lookups per visitor per minute, from `TRAINER_LOOKUPS_PER_MINUTE`.
    pub trainer_lookups_per_minute: u32,
    /// Whether to identify visitors by `X-Forwarded-For`, from `TRUST_FORWARDED_FOR`.
    pub trust_forwarded_for: bool,
    /// Built frontend to serve, from `FRONTEND_DIR`. `None` serves only the API.
    pub frontend_dir: Option<PathBuf>,
    /// Redis to share caches with other instances, from `REDIS_URL`. `None` keeps everything local.
    pub redis_url: Option<RedisUrl>,
    /// Secret for hashing trainer IDs in Redis keys, from `REDIS_KEY_SECRET`. Set whenever
    /// `redis_url` is.
    pub redis_key_secret: Option<KeySecret>,
    /// Address to serve Prometheus metrics on, from `METRICS_ADDR`. `None` records no metrics.
    pub metrics_addr: Option<SocketAddr>,
}

/// Invalid or missing settings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// `UMAMOE_API_KEY` is missing or blank.
    #[error("UMAMOE_API_KEY is not set")]
    MissingApiKey,
    /// `HOST` isn't an IP address.
    #[error("HOST must be an IP address, got {0:?}")]
    InvalidHost(String),
    /// `PORT` isn't a port number from 1 to 65535.
    #[error("PORT must be a port number from 1 to 65535, got {0:?}")]
    InvalidPort(String),
    /// `FRONTEND_DIR` isn't a directory containing `index.html`.
    #[error("FRONTEND_DIR must be a directory containing index.html, got {0:?}")]
    InvalidFrontendDir(String),
    /// `REDIS_URL` isn't a `redis://` or `rediss://` URL. The value isn't shown, as it can hold a
    /// password.
    #[error("REDIS_URL must be a redis:// or rediss:// URL")]
    InvalidRedisUrl,
    /// `REDIS_URL` is set without `REDIS_KEY_SECRET`.
    #[error("REDIS_KEY_SECRET is required when REDIS_URL is set")]
    MissingRedisKeySecret,
    /// `REDIS_KEY_SECRET` is shorter than 32 characters. The value isn't shown.
    #[error("REDIS_KEY_SECRET must be at least 32 characters")]
    InvalidRedisKeySecret,
    /// `METRICS_ADDR` isn't an IP address with a port from 1 to 65535.
    #[error("METRICS_ADDR must be an IP address and port, such as 127.0.0.1:9464, got {0:?}")]
    InvalidMetricsAddr(String),
    /// A true-or-false setting is something else.
    #[error("{name} must be true or false, got {value:?}")]
    InvalidBool {
        /// Variable name.
        name: &'static str,
        /// Value that was set.
        value: String,
    },
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
        let scan_interval = match get("SCAN_INTERVAL_MINUTES") {
            None => None,
            Some(_) => Some(Duration::from_secs(
                u64::from(number("SCAN_INTERVAL_MINUTES", 0)?) * 60,
            )),
        };
        let detail_cache = match get("DETAIL_CACHE_MINUTES") {
            None => None,
            Some(_) => Some(Duration::from_secs(
                u64::from(number("DETAIL_CACHE_MINUTES", 0)?) * 60,
            )),
        };
        let match_defaults = MatchSettings::default();
        let matching = MatchSettings {
            shortlist_size: number("SHORTLIST_SIZE", match_defaults.shortlist_size as u32)?
                as usize,
            fetch_budget: number("REQUEST_FETCH_BUDGET", match_defaults.fetch_budget as u32)?
                as usize,
        };
        let trainer_cache = Duration::from_secs(
            u64::from(number(
                "TRAINER_CACHE_MINUTES",
                (DEFAULT_PACE_TTL.as_secs() / 60) as u32,
            )?) * 60,
        );
        let rate_limit_per_minute = number("RATE_LIMIT_PER_MINUTE", DEFAULT_RATE_LIMIT_PER_MINUTE)?;
        let trainer_lookups_per_minute = number(
            "TRAINER_LOOKUPS_PER_MINUTE",
            DEFAULT_TRAINER_LOOKUPS_PER_MINUTE,
        )?;
        let trust_forwarded_for = match get("TRUST_FORWARDED_FOR").as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            Some(value) => {
                return Err(ConfigError::InvalidBool {
                    name: "TRUST_FORWARDED_FOR",
                    value: value.to_owned(),
                })
            }
        };
        let frontend_dir = match get("FRONTEND_DIR") {
            None => None,
            Some(value) => {
                let dir = PathBuf::from(&value);
                if !dir.join("index.html").is_file() {
                    return Err(ConfigError::InvalidFrontendDir(value));
                }
                Some(dir)
            }
        };
        let redis_url = match get("REDIS_URL") {
            None => None,
            Some(value) => Some(RedisUrl::parse(&value).ok_or(ConfigError::InvalidRedisUrl)?),
        };
        let redis_key_secret = match (&redis_url, get("REDIS_KEY_SECRET")) {
            (None, _) => None,
            (Some(_), None) => return Err(ConfigError::MissingRedisKeySecret),
            (Some(_), Some(value)) => {
                Some(KeySecret::parse(&value).ok_or(ConfigError::InvalidRedisKeySecret)?)
            }
        };
        let metrics_addr = match get("METRICS_ADDR") {
            None => None,
            Some(value) => Some(
                value
                    .parse::<SocketAddr>()
                    .ok()
                    .filter(|a| a.port() > 0)
                    .ok_or(ConfigError::InvalidMetricsAddr(value))?,
            ),
        };
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
            detail_cache,
            matching,
            trainer_cache,
            rate_limit_per_minute,
            trainer_lookups_per_minute,
            trust_forwarded_for,
            frontend_dir,
            redis_url,
            redis_key_secret,
            metrics_addr,
        })
    }
}

#[cfg(test)]
mod tests;
