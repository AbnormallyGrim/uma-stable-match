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
mod scanner;
mod scheduler;

pub use app::{app, router, AppState};
pub use cache::{Cache, CacheStatus, ScanFailure, Snapshot};
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
    elapsed_days, game_month_start, jst, raw_rate, ClubPool, ListedClub, DEFAULT_SHORTLIST_SIZE,
    TARGET_MEDIAN_RATIO,
};
pub use recommendations::{MatchSettings, DEFAULT_FETCH_BUDGET};
pub use scanner::{scan, Scan, ScanConfig, ScanError};
pub use scheduler::run_scheduler;
