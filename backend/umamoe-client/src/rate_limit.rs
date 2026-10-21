//! Client-side rate limiting.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::{sleep_until, Instant};

use crate::config::BackoffConfig;

/// Which lane a client's requests wait in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Priority {
    /// Someone is waiting for the answer. Always goes first.
    #[default]
    Interactive,
    /// Prefetching and other work that can wait.
    Background,
}

#[derive(Debug, Default)]
struct State {
    last_grant: Option<Instant>,
    next_ticket: u64,
    interactive: VecDeque<u64>,
    background: VecDeque<u64>,
    gap: Duration,
    paused_until: Option<Instant>,
    last_change: Option<Instant>,
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

/// Spaces requests at least `interval` apart. Interactive requests go first, and each lane keeps
/// the order its requests asked in.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    interval: Duration,
    backoff: BackoffConfig,
    state: Mutex<State>,
    changed: Notify,
}

impl RateLimiter {
    #[cfg(test)]
    pub(crate) fn new(interval: Duration) -> Self {
        Self::with_backoff(interval, BackoffConfig::default())
    }

    pub(crate) fn with_backoff(interval: Duration, backoff: BackoffConfig) -> Self {
        record_rate(interval);
        Self {
            interval,
            backoff,
            state: Mutex::new(State {
                gap: interval,
                ..State::default()
            }),
            changed: Notify::new(),
        }
    }

    pub(crate) fn throttled(&self, retry_after: Option<Duration>) {
        let mut state = self.lock();
        let now = Instant::now();
        let pausing = state.paused_until.is_some_and(|until| until > now);
        if let Some(wait) = retry_after {
            let until = now + wait;
            state.paused_until = Some(state.paused_until.map_or(until, |u| u.max(until)));
        }
        if !self.interval.is_zero() {
            self.recover(&mut state, now);
            if !pausing {
                state.gap = (state.gap * 2).min(self.backoff.max_interval.max(self.interval));
                record_rate(state.gap);
            }
            state.last_change = Some(state.paused_until.map_or(now, |until| until.max(now)));
        }
        drop(state);
        self.changed.notify_waiters();
    }

    fn recover(&self, state: &mut State, now: Instant) {
        if state.gap <= self.interval {
            return;
        }
        let Some(since) = state.last_change else {
            return;
        };
        let every = self.backoff.recovery_every;
        if every.is_zero() {
            state.gap = self.interval;
            record_rate(state.gap);
            return;
        }
        let steps =
            (now.saturating_duration_since(since).as_secs_f64() / every.as_secs_f64()).floor();
        if steps < 1.0 {
            return;
        }
        let rate = 1.0 / state.gap.as_secs_f64() + steps * self.backoff.recovery_step;
        state.gap = Duration::from_secs_f64(1.0 / rate).max(self.interval);
        state.last_change = Some(since + every.mul_f64(steps));
        record_rate(state.gap);
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
                    self.recover(&mut state, now);
                    let gap = state.gap;
                    let mut slot = state.last_grant.map_or(now, |t| (t + gap).max(now));
                    if let Some(until) = state.paused_until {
                        slot = slot.max(until);
                    }
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

fn record_rate(gap: Duration) {
    if !gap.is_zero() {
        metrics::gauge!("umamoe_allowed_rate").set(1.0 / gap.as_secs_f64());
    }
}

// Takes a cancelled request out of its lane so it doesn't hold up the ones behind it.
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
