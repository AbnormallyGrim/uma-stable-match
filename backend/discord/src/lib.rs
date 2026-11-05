#![doc = include_str!("../README.md")]

mod auth;
mod bot;
mod config;
mod error;
mod gateway;
mod guild_access;
mod interactions;
mod invites;
mod leaders;
mod names;
mod oauth;
mod opt_in;
mod rate_limit;
mod rest;
mod settings;
mod store;
mod token;
mod trainer_store;
mod trainers;

pub use auth::{
    ACCOUNT_PATH, APPLICATIONS_ME_PATH, APPLICATIONS_PAGE, CALLBACK_PATH, LOGIN_PATH,
    MAX_SESSION_AGE, MAX_STATE_AGE, SIGN_OUT_PATH,
};
pub use bot::Discord;
pub use config::{
    ApplicationId, BotToken, ClientSecret, DiscordConfig, KeySecret, PublicKey, PublicUrl,
    RateLimitConfig, SigningKey, DEFAULT_API_BASE, DEFAULT_AUTHORIZE_URL,
};
pub use error::Error;
pub use interactions::{router, INTERACTIONS_PATH, MAX_TIMESTAMP_SKEW};
pub use invites::{
    invite_candidates, resolve_invite, servers_in_description, MAX_INVITE_CANDIDATES,
};
pub use leaders::MY_CLUB_PATH;
pub use names::NAME_TTL;
pub use opt_in::{
    ClubLookup, ClubSummary, LookupError, OptIn, BOT_PERMISSIONS, OPT_IN_PATH, UNLINK_PATH,
};
pub use reqwest::StatusCode;
pub use rest::Rest;
pub use settings::SETTINGS_PATH;
pub use store::{ClubRecord, ClubStore, Departure, SettingsUpdate, StoreError};
pub use trainer_store::{
    Action, CheckState, LinkOutcome, LinkedTrainer, Pending, PendingLink, TrainerStore,
    KEY_LIFETIME, MAX_LINKED_TRAINERS,
};
pub use trainers::{
    TrainerLinks, TrainerLookup, TrainerSummary, CHECK_EVERY, CHECK_FOR, LINK_PATH, TRAINERS_PATH,
    TRAINER_PATH, VERIFICATION_PATH,
};
