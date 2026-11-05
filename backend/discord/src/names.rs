//! Discord display names, looked up as the bot and held in memory for a short while.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::rest::Rest;

/// How long a looked-up name is reused.
pub const NAME_TTL: Duration = Duration::from_secs(10 * 60);

/// Most names held at once; past it, expired ones are dropped, then all of them.
const MAX_NAMES: usize = 10_000;

#[derive(Deserialize)]
struct User {
    username: String,
    global_name: Option<String>,
}

/// Recently looked-up display names, by Discord ID.
#[derive(Debug, Default)]
pub(crate) struct Names {
    names: Mutex<HashMap<u64, (String, Instant)>>,
}

impl Names {
    /// `user`'s display name, or their username without one; `None` if Discord can't be asked.
    pub(crate) async fn get(&self, rest: &Rest, user: u64) -> Option<String> {
        if let Some((name, at)) = self.lock().get(&user) {
            if at.elapsed() < NAME_TTL {
                return Some(name.clone());
            }
        }
        let found: User = match rest.get(&format!("/users/{user}")).await {
            Ok(found) => found,
            Err(err) => {
                tracing::warn!(error = %err, "couldn't look up a Discord name");
                return None;
            }
        };
        let name = found
            .global_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(found.username);
        let mut names = self.lock();
        if names.len() >= MAX_NAMES {
            names.retain(|_, (_, at)| at.elapsed() < NAME_TTL);
            if names.len() >= MAX_NAMES {
                names.clear();
            }
        }
        names.insert(user, (name.clone(), Instant::now()));
        Some(name)
    }

    /// Drops `user`'s name.
    pub(crate) fn forget(&self, user: u64) {
        self.lock().remove(&user);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, (String, Instant)>> {
        self.names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
