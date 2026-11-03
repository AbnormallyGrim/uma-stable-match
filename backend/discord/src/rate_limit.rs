//! Discord's per-route and global rate limits.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::Method;
use tokio::time::Instant;

use crate::error::Error;

/// Buckets kept before expired ones are dropped.
const MAX_BUCKETS: usize = 1024;

/// What Discord last said about each route, and any global pause.
#[derive(Debug, Default)]
pub(crate) struct RateLimits {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    buckets: HashMap<String, Bucket>,
    global_until: Option<Instant>,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    remaining: u32,
    reset_at: Instant,
}

impl RateLimits {
    /// Waits until `route` may be called, or fails if that's longer than `max_wait`.
    pub(crate) async fn wait(&self, route: &str, max_wait: Duration) -> Result<(), Error> {
        let delay = self.delay(route);
        if delay.is_zero() {
            return Ok(());
        }
        if delay > max_wait {
            return Err(Error::RateLimited(delay));
        }
        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// How long `route` must wait now.
    pub(crate) fn delay(&self, route: &str) -> Duration {
        let now = Instant::now();
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let global = state
            .global_until
            .map_or(Duration::ZERO, |until| until.saturating_duration_since(now));
        let bucket = state
            .buckets
            .get(route)
            .filter(|b| b.remaining == 0)
            .map_or(Duration::ZERO, |b| {
                b.reset_at.saturating_duration_since(now)
            });
        global.max(bucket)
    }

    /// Records the `X-RateLimit-Remaining` and `X-RateLimit-Reset-After` headers for `route`.
    pub(crate) fn update(&self, route: &str, headers: &HeaderMap) {
        let (Some(remaining), Some(reset_after)) = (
            header(headers, "x-ratelimit-remaining").and_then(|v| v.parse().ok()),
            header(headers, "x-ratelimit-reset-after").and_then(seconds),
        ) else {
            return;
        };
        self.set(route, remaining, reset_after);
    }

    /// Records a 429: `route`, or every route if `global`, waits `wait`.
    pub(crate) fn limited(&self, route: &str, wait: Duration, global: bool) {
        if global {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.global_until = Some(Instant::now() + wait);
        } else {
            self.set(route, 0, wait);
        }
    }

    fn set(&self, route: &str, remaining: u32, reset_after: Duration) {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.buckets.len() >= MAX_BUCKETS {
            state.buckets.retain(|_, b| b.reset_at > now);
        }
        state.buckets.insert(
            route.to_owned(),
            Bucket {
                remaining,
                reset_at: now + reset_after,
            },
        );
    }
}

/// How long a 429 asks to wait, from its JSON `retry_after` or the `Retry-After` header, and
/// whether the limit is global.
pub(crate) fn retry_after(headers: &HeaderMap, body: &[u8]) -> (Duration, bool) {
    #[derive(serde::Deserialize)]
    struct Limited {
        retry_after: f64,
        #[serde(default)]
        global: bool,
    }
    let header_global = header(headers, "x-ratelimit-global").is_some_and(|v| v == "true");
    match serde_json::from_slice::<Limited>(body) {
        Ok(limited) if limited.retry_after.is_finite() && limited.retry_after >= 0.0 => (
            Duration::from_secs_f64(limited.retry_after),
            limited.global || header_global,
        ),
        _ => (
            header(headers, RETRY_AFTER.as_str())
                .and_then(seconds)
                .unwrap_or(Duration::from_secs(1)),
            header_global,
        ),
    }
}

/// The rate-limit key for a request: the method and path, with IDs replaced except the major
/// ones (channel, guild and webhook) that Discord limits separately.
pub(crate) fn route_key(method: &Method, path: &str) -> String {
    let path = path.split('?').next().unwrap_or_default();
    let mut key = method.as_str().to_owned();
    let mut previous = "";
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        key.push('/');
        let is_id = segment.bytes().all(|b| b.is_ascii_digit());
        if is_id && !matches!(previous, "channels" | "guilds" | "webhooks") {
            key.push_str(":id");
        } else {
            key.push_str(segment);
        }
        previous = segment;
    }
    key
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok().map(str::trim)
}

fn seconds(value: &str) -> Option<Duration> {
    let seconds: f64 = value.parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then(|| Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests;
