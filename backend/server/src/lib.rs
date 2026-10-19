#![doc = include_str!("../README.md")]

mod api_error;
mod app;
mod cache;
mod config;
mod frontend;
mod limits;
mod pace;
mod recommendations;
mod scanner;
mod scheduler;

pub use app::{app, router, AppState};
pub use cache::{Cache, CacheStatus, ScanFailure, Snapshot};
pub use config::{
    Config, ConfigError, DEFAULT_HOST, DEFAULT_PORT, DEFAULT_RATE_LIMIT_PER_MINUTE,
    DEFAULT_SCAN_INTERVAL_MINUTES, DEFAULT_TRAINER_LOOKUPS_PER_MINUTE,
};
pub use frontend::CONTENT_SECURITY;
pub use limits::RateLimits;
pub use pace::{PaceError, PaceLookup, DEFAULT_PACE_TTL};
pub use scanner::{scan, Scan, ScanConfig, ScanError};
pub use scheduler::run_scheduler;
