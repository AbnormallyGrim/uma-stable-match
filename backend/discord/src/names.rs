//! Discord display names and avatars, looked up as the bot and held in memory for a short while.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::rest::Rest;

/// How long a looked-up profile is reused.
pub const NAME_TTL: Duration = Duration::from_secs(10 * 60);

/// Most profiles held at once; past it, expired ones are dropped, then all of them.
const MAX_NAMES: usize = 10_000;

#[derive(Deserialize)]
struct User {
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

/// What the site shows of a Discord user.
#[derive(Clone, Debug)]
pub(crate) struct Profile {
    /// Their display name, or their username without one.
    pub(crate) name: String,
    /// Their avatar hash, `None` for Discord's default avatar or one that isn't well formed.
    pub(crate) avatar: Option<String>,
}

/// Recently looked-up profiles, by Discord ID.
#[derive(Debug, Default)]
pub(crate) struct Names {
    names: Mutex<HashMap<u64, (Profile, Instant)>>,
}

impl Names {
    /// `user`'s profile; `None` if Discord can't be asked.
    pub(crate) async fn get(&self, rest: &Rest, user: u64) -> Option<Profile> {
        if let Some((profile, at)) = self.lock().get(&user) {
            if at.elapsed() < NAME_TTL {
                return Some(profile.clone());
            }
        }
        let found: User = match rest.get(&format!("/users/{user}")).await {
            Ok(found) => found,
            Err(err) => {
                tracing::warn!(error = %err, "couldn't look up a Discord name");
                return None;
            }
        };
        let profile = Profile {
            name: found
                .global_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(found.username),
            avatar: found.avatar.filter(|hash| is_avatar_hash(hash)),
        };
        let mut names = self.lock();
        if names.len() >= MAX_NAMES {
            names.retain(|_, (_, at)| at.elapsed() < NAME_TTL);
            if names.len() >= MAX_NAMES {
                names.clear();
            }
        }
        names.insert(user, (profile.clone(), Instant::now()));
        Some(profile)
    }

    /// Drops `user`'s profile.
    pub(crate) fn forget(&self, user: u64) {
        self.lock().remove(&user);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, (Profile, Instant)>> {
        self.names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Whether `hash` looks like a Discord avatar hash (32 hex digits, `a_` first when animated), so
/// it's safe to put in a CDN URL.
fn is_avatar_hash(hash: &str) -> bool {
    let digits = hash.strip_prefix("a_").unwrap_or(hash);
    digits.len() == 32
        && digits
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests;
