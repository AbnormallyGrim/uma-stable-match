#![doc = include_str!("../README.md")]

mod app;
mod cache;
mod config;
mod pace;
mod recommendations;
mod scanner;
mod scheduler;

pub use app::{router, AppState};
pub use cache::{Cache, CacheStatus, ScanFailure, Snapshot};
pub use config::{Config, ConfigError, DEFAULT_BIND_ADDR, DEFAULT_SCAN_INTERVAL_MINUTES};
pub use pace::{PaceError, PaceLookup, DEFAULT_PACE_TTL};
pub use scanner::{scan, Scan, ScanConfig, ScanError};
pub use scheduler::run_scheduler;
