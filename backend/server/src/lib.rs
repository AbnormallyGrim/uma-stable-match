#![doc = include_str!("../README.md")]

mod app;
mod config;

pub use app::router;
pub use config::{Config, ConfigError, DEFAULT_BIND_ADDR};
