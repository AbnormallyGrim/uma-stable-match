#![doc = include_str!("../README.md")]

mod auth;
mod bot;
mod config;
mod error;
mod guild_access;
mod interactions;
mod invites;
mod oauth;
mod opt_in;
mod rate_limit;
mod rest;
mod settings;
mod store;
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
pub use invites::{
    invite_candidates, resolve_invite, servers_in_description, MAX_INVITE_CANDIDATES,
};
pub use opt_in::{
    ClubLookup, ClubSummary, LookupError, OptIn, BOT_PERMISSIONS, OPT_IN_PATH, UNLINK_PATH,
};
pub use reqwest::StatusCode;
pub use rest::Rest;
pub use settings::{MAX_LEADER_ROLES, SETTINGS_PATH};
pub use store::{ClubRecord, ClubStore, Departure, SettingsUpdate, StoreError};
