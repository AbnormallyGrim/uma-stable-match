#![doc = include_str!("../README.md")]

mod api_error;
mod app;
mod cache;
mod calibration;
mod club;
mod config;
mod details;
mod features;
mod frontend;
mod human;
mod limits;
mod logging;
mod pace;
mod pool;
mod priors;
mod recommendations;
mod refresher;
mod run;
mod scanner;
mod scheduler;
mod shared;
mod source;
mod telemetry;
mod traces;

pub use app::{app, app_with_features, router, status_router, AppState};
pub use cache::{Cache, CacheStatus, ScanFailure, ScanPhase, ScanProgress, Snapshot};
pub use calibration::{club_median, Calibration, MIN_CALIBRATION_SAMPLES};
pub use config::{
    Config, ConfigError, UmaMoeConfig, DEFAULT_DETAIL_CACHE_MB, DEFAULT_HOST, DEFAULT_PORT,
    DEFAULT_RATE_LIMIT_PER_MINUTE, DEFAULT_SCAN_INTERVAL_MINUTES,
    DEFAULT_TRAINER_LOOKUPS_PER_MINUTE,
};
pub use details::{club_weight, BackgroundLoader, ClubDetails, DetailConfig, SharedPool};
pub use features::Features;
pub use frontend::CONTENT_SECURITY;
pub use human::{HumanCheck, SigningKey, TurnstileSecret, TOKEN_LIFETIME};
pub use limits::RateLimits;
pub use logging::{init_logging, InvalidLogFormat, LogFormat};
pub use pace::{
    PaceError, PaceLookup, TrainerPace, DEFAULT_PACE_TTL, MAX_PRIOR_RATIO, TRAINER_BLEND_DAYS,
    TRAINER_PRIOR_K,
};
pub use pool::{
    elapsed_days, game_month, game_month_start, is_early_month, join_style_bit, jst,
    last_month_rate, mentions_discord, previous_game_month, previous_game_month_days, raw_rate,
    ClubPool, ListedClub, Period, ShortlistFilter, DEFAULT_SHORTLIST_SIZE, EARLY_MONTH_DAYS,
    TARGET_MEDIAN_RATIO,
};
pub use priors::{CLUB_BLEND_DAYS, CLUB_PRIOR_K};
pub use recommendations::{MatchSettings, DEFAULT_FETCH_BUDGET};
pub use refresher::{RefreshConfig, RefreshStatus, Refresher, StoredTtl, TierStatus};
pub use run::{run, RunError};
pub use scanner::{scan, Scan, ScanConfig, ScanError};
pub use scheduler::run_scheduler;
pub use shared::{club_key, KeySecret, RedisUrl, SharedCache, SharedCacheError};
pub use source::{ClubSource, Source, SourceError, SourceFuture, UmaMoeSource};
pub use telemetry::{install_metrics, metrics_router};
pub use traces::{TraceExport, TraceExportError};
