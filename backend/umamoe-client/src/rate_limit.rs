//! Client-side rate limiting.

use std::sync::Mutex;
use std::time::Duration;

use tokio::time::{sleep_until, Instant};

/// Spaces requests at least `interval` apart, in the order they ask.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    interval: Duration,
    next_slot: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_slot: Mutex::new(None),
        }
    }

    /// Waits for the next free slot.
    pub(crate) async fn acquire(&self) {
        let slot = {
            let mut next = self.next_slot.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let slot = next.map_or(now, |n| n.max(now));
            *next = Some(slot + self.interval);
            slot
        };
        sleep_until(slot).await;
    }
}

#[cfg(test)]
mod tests;
