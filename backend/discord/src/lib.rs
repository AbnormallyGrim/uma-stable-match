#![doc = include_str!("../README.md")]

mod auth;
mod bot;
mod config;
mod error;
mod interactions;
mod oauth;
mod rate_limit;
mod rest;
mod token;

pub use auth::{
    CALLBACK_PATH, LEADERS_ME_PATH, LOGIN_PATH, MAX_SESSION_AGE, MAX_STATE_AGE, SIGN_OUT_PATH,
};
pub use bot::Discord;
pub use config::{
    ApplicationId, BotToken, ClientSecret, DiscordConfig, PublicKey, PublicUrl, RateLimitConfig,
    SigningKey, DEFAULT_API_BASE, DEFAULT_AUTHORIZE_URL,
};
pub use error::Error;
pub use interactions::{router, INTERACTIONS_PATH, MAX_TIMESTAMP_SKEW};
pub use reqwest::StatusCode;
pub use rest::Rest;
