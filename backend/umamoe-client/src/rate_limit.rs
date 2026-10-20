//! Client-side rate limiting.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::{sleep_until, Instant};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Priority {
    #[default]
    Interactive,
    Background,
}

#[derive(Debug, Default)]
struct State {
    last_grant: Option<Instant>,
    next_ticket: u64,
    interactive: VecDeque<u64>,
    background: VecDeque<u64>,
}

impl State {
    fn lane(&mut self, priority: Priority) -> &mut VecDeque<u64> {
        match priority {
            Priority::Interactive => &mut self.interactive,
            Priority::Background => &mut self.background,
        }
    }

    fn head(&self) -> Option<u64> {
        self.interactive
            .front()
            .or_else(|| self.background.front())
            .copied()
    }
}

/// Spaces requests at least `interval` apart, in the order they ask.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    interval: Duration,
    state: Mutex<State>,
    changed: Notify,
}

impl RateLimiter {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: Mutex::new(State::default()),
            changed: Notify::new(),
        }
    }

    /// Waits for the next free slot.
    pub(crate) async fn acquire(&self, priority: Priority) {
        let mut waiter = Waiter {
            limiter: self,
            priority,
            ticket: None,
        };
        let ticket = {
            let mut state = self.lock();
            let ticket = state.next_ticket;
            state.next_ticket += 1;
            state.lane(priority).push_back(ticket);
            ticket
        };
        waiter.ticket = Some(ticket);
        self.changed.notify_waiters();

        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();

            let slot = {
                let mut state = self.lock();
                if state.head() != Some(ticket) {
                    None
                } else {
                    let now = Instant::now();
                    let slot = state
                        .last_grant
                        .map_or(now, |t| (t + self.interval).max(now));
                    if slot <= now {
                        state.lane(priority).pop_front();
                        state.last_grant = Some(now);
                        waiter.ticket = None;
                        drop(state);
                        self.changed.notify_waiters();
                        return;
                    }
                    Some(slot)
                }
            };

            match slot {
                Some(slot) => {
                    tokio::select! {
                        _ = sleep_until(slot) => {}
                        _ = &mut changed => {}
                    }
                }
                None => changed.await,
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

struct Waiter<'a> {
    limiter: &'a RateLimiter,
    priority: Priority,
    ticket: Option<u64>,
}

impl Drop for Waiter<'_> {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket {
            self.limiter
                .lock()
                .lane(self.priority)
                .retain(|t| *t != ticket);
            self.limiter.changed.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests;
