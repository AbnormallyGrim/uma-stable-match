#![doc = include_str!("../README.md")]

mod client;
mod config;
mod error;
mod rate_limit;
mod requests;
mod retry;
mod types;

pub use client::Client;
pub use config::{ApiKey, BackoffConfig, ClientConfig, RetryConfig, DEFAULT_BASE_URL};
pub use error::{ApiErrorBody, Error};
pub use rate_limit::Priority;
pub use requests::{
    CircleLookup, CircleRequest, CircleSort, GainsSort, ListCirclesRequest, RankingsGainsRequest,
    SortDir,
};
pub use reqwest::StatusCode;
pub use types::{
    Circle, CircleListResponse, CircleMember, CircleResponse, GainsRanking, GainsRankingsResponse,
};
