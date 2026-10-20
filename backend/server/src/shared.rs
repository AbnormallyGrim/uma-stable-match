//! Optional cache shared between server instances.

use std::fmt;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::pool::Period;

const KEY_PREFIX: &str = "usm:v1";

/// A `redis://` or `rediss://` URL. `Debug` output hides it, since it can hold a password.
#[derive(Clone, PartialEq, Eq)]
pub struct RedisUrl(String);

impl RedisUrl {
    /// The URL, or `None` without a `redis://` or `rediss://` scheme and a host.
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value
            .strip_prefix("redis://")
            .or_else(|| value.strip_prefix("rediss://"))?;
        (!rest.is_empty()).then(|| Self(value.to_owned()))
    }

    /// The URL as given, credentials included.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RedisUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RedisUrl(<redacted>)")
    }
}

/// A cache shared between server instances, behind each instance's own cache. The default
/// stores nothing.
#[derive(Debug, Clone, Default)]
pub struct SharedCache {
    backend: Backend,
}

#[derive(Debug, Clone, Default)]
enum Backend {
    #[default]
    None,
}

impl SharedCache {
    /// A shared cache that stores nothing.
    pub fn none() -> Self {
        Self::default()
    }

    /// Whether values are actually shared.
    pub fn is_enabled(&self) -> bool {
        match self.backend {
            Backend::None => false,
        }
    }

    /// The stored value, or `None` when it's missing, expired or unreadable.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        match self.backend {
            Backend::None => {
                let _ = key;
                None
            }
        }
    }

    /// Stores a value until `ttl` passes. Failures are ignored, as the value can be fetched again.
    pub async fn set<T: Serialize + Sync>(&self, key: &str, value: &T, ttl: Duration) {
        match self.backend {
            Backend::None => {
                let _ = (key, value, ttl);
            }
        }
    }
}

/// Shared-cache key for a club's details in a month (`None` = the current game month).
pub fn club_key(circle_id: u64, period: Option<Period>) -> String {
    format!("{KEY_PREFIX}:club:{circle_id}:{}", period_part(period))
}

/// Shared-cache key for a trainer's pace in a month (`None` = the current game month).
pub fn pace_key(viewer_id: u64, period: Option<Period>) -> String {
    format!("{KEY_PREFIX}:pace:{viewer_id}:{}", period_part(period))
}

fn period_part(period: Option<Period>) -> String {
    match period {
        None => "current".to_owned(),
        Some((year, month)) => format!("{year:04}-{month:02}"),
    }
}

#[cfg(test)]
mod tests;
