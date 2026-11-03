#![doc = include_str!("../README.md")]

mod bot;
mod config;
mod error;
mod interactions;
mod rate_limit;
mod rest;

pub use bot::Discord;
pub use config::{
    ApplicationId, BotToken, ClientSecret, DiscordConfig, PublicKey, RateLimitConfig,
    DEFAULT_API_BASE,
};
pub use error::Error;
pub use interactions::{router, INTERACTIONS_PATH, MAX_TIMESTAMP_SKEW};
pub use reqwest::StatusCode;
pub use rest::Rest;
