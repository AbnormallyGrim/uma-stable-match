#![doc = include_str!("../README.md")]

mod api_error;
mod app;
mod cache;
mod calibration;
mod config;
mod details;
mod frontend;
mod limits;
mod pace;
mod pool;
mod recommendations;
mod refresher;
mod scanner;
mod scheduler;
mod shared;
mod telemetry;

pub use app::{app, router, AppState};
pub use cache::{Cache, CacheStatus, ScanFailure, ScanPhase, ScanProgress, Snapshot};
pub use calibration::{club_median, Calibration, MIN_CALIBRATION_SAMPLES};
pub use config::{
    Config, ConfigError, DEFAULT_HOST, DEFAULT_PORT, DEFAULT_RATE_LIMIT_PER_MINUTE,
    DEFAULT_SCAN_INTERVAL_MINUTES, DEFAULT_TRAINER_LOOKUPS_PER_MINUTE,
};
pub use details::{BackgroundLoader, ClubDetails, DetailConfig, SharedPool};
pub use frontend::CONTENT_SECURITY;
pub use limits::RateLimits;
pub use pace::{PaceError, PaceLookup, DEFAULT_PACE_TTL};
pub use pool::{
    elapsed_days, game_month, game_month_start, is_early_month, jst, last_month_rate,
    previous_game_month, previous_game_month_days, raw_rate, ClubPool, ListedClub, Period,
    DEFAULT_SHORTLIST_SIZE, EARLY_MONTH_DAYS, TARGET_MEDIAN_RATIO,
};
pub use recommendations::{MatchSettings, DEFAULT_FETCH_BUDGET};
pub use refresher::{RefreshConfig, Refresher};
pub use scanner::{scan, Scan, ScanConfig, ScanError};
pub use scheduler::run_scheduler;
pub use shared::{club_key, KeySecret, RedisUrl, SharedCache, SharedCacheError};
pub use telemetry::{install_metrics, metrics_router};
