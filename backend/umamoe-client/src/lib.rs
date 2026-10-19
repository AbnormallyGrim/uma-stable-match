#![doc = include_str!("../README.md")]

mod client;
mod config;
mod error;
mod requests;
mod types;

pub use client::Client;
pub use config::{ApiKey, ClientConfig, DEFAULT_BASE_URL};
pub use error::{ApiErrorBody, Error};
pub use requests::{CircleLookup, CircleRequest};
pub use reqwest::StatusCode;
pub use types::{Circle, CircleListResponse, CircleMember, CircleResponse};
