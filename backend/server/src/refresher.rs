use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Utc};
use tokio::sync::Notify;
use uma_stable_match_umamoe_client::Priority;

use crate::details::ClubDetails;

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshConfig {
    pub every: Option<Duration>,
    pub margin: Duration,
    pub recheck: Duration,
    pub retry: Duration,
    pub fast_ranks: u32,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            every: None,
            margin: Duration::from_secs(30 * 60),
            recheck: Duration::from_secs(60 * 60),
            retry: Duration::from_secs(5 * 60),
            fast_ranks: 5_000,
        }
    }
}

impl RefreshConfig {
    pub fn next_refresh(
        &self,
        last_updated: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> DateTime<Utc> {
        if let Some(every) = self.every {
            return now + every;
        }
        match last_updated.map(|at| at + DAY + self.margin) {
            Some(due) if due > now => due,
            _ => now + self.recheck,
        }
    }

    fn is_current(&self, last_updated: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
        self.every.is_none() && last_updated.is_some_and(|at| at + DAY + self.margin > now)
    }
}

pub(crate) struct Refreshing {
    pub(crate) config: RefreshConfig,
    schedule: Mutex<Schedule>,
    wake: Notify,
}

impl Refreshing {
    pub(crate) fn new(config: RefreshConfig) -> Self {
        Self {
            config,
            schedule: Mutex::new(Schedule::default()),
            wake: Notify::new(),
        }
    }

    pub(crate) fn replace(
        &self,
        clubs: impl IntoIterator<Item = (u64, u32)>,
        keep: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) {
        lock(&self.schedule).replace(clubs, keep, now);
        self.wake.notify_one();
    }

    pub(crate) fn loaded(&self, circle_id: u64, rank: u32, due: DateTime<Utc>) {
        if lock(&self.schedule).insert_new(circle_id, rank, due) {
            self.wake.notify_one();
        }
    }

    pub(crate) fn due_at(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        lock(&self.schedule).due_at(circle_id)
    }

    pub(crate) fn len(&self) -> usize {
        lock(&self.schedule).len()
    }
}

#[derive(Debug, Default)]
pub(crate) struct Schedule {
    order: BTreeSet<(DateTime<Utc>, u32, u64)>,
    slots: HashMap<u64, (DateTime<Utc>, u32)>,
}

impl Schedule {
    pub(crate) fn replace(
        &mut self,
        clubs: impl IntoIterator<Item = (u64, u32)>,
        keep: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) {
        let dropped: Vec<u64> = self.slots.keys().copied().filter(|id| !keep(*id)).collect();
        for circle_id in dropped {
            self.remove(circle_id);
        }
        for (circle_id, rank) in clubs {
            let due = self.remove(circle_id).map_or(now, |(due, _)| due);
            self.insert(circle_id, rank, due);
        }
    }

    pub(crate) fn insert_new(&mut self, circle_id: u64, rank: u32, due: DateTime<Utc>) -> bool {
        if self.slots.contains_key(&circle_id) {
            return false;
        }
        self.insert(circle_id, rank, due);
        true
    }

    pub(crate) fn reschedule(&mut self, circle_id: u64, due: DateTime<Utc>) {
        if let Some((_, rank)) = self.remove(circle_id) {
            self.insert(circle_id, rank, due);
        }
    }

    pub(crate) fn next(&self) -> Option<(DateTime<Utc>, u32, u64)> {
        self.order.first().copied()
    }

    pub(crate) fn due_at(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        self.slots.get(&circle_id).map(|(due, _)| *due)
    }

    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    fn insert(&mut self, circle_id: u64, rank: u32, due: DateTime<Utc>) {
        self.order.insert((due, rank, circle_id));
        self.slots.insert(circle_id, (due, rank));
    }

    fn remove(&mut self, circle_id: u64) -> Option<(DateTime<Utc>, u32)> {
        let (due, rank) = self.slots.remove(&circle_id)?;
        self.order.remove(&(due, rank, circle_id));
        Some((due, rank))
    }
}

pub struct Refresher {
    details: ClubDetails,
    refreshing: Arc<Refreshing>,
}

impl Refresher {
    pub(crate) fn new(details: ClubDetails, refreshing: Arc<Refreshing>) -> Self {
        Self {
            details,
            refreshing,
        }
    }

    pub async fn run(self) {
        loop {
            let now = utc_now();
            let next = lock(&self.refreshing.schedule).next();
            match next {
                None => self.refreshing.wake.notified().await,
                Some((due, _, _)) if due > now => {
                    let wait = due.signed_duration_since(now).to_std().unwrap_or_default();
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = self.refreshing.wake.notified() => {}
                    }
                }
                Some((_, rank, circle_id)) => {
                    let took = self.refresh(circle_id, now).await;
                    if let Some(took) = took.filter(|_| rank > self.refreshing.config.fast_ranks) {
                        tokio::time::sleep(took).await;
                    }
                }
            }
        }
    }

    async fn refresh(&self, circle_id: u64, now: DateTime<Utc>) -> Option<Duration> {
        let config = self.refreshing.config;
        let cached = self.details.last_updated(circle_id).await;
        if config.is_current(cached, now) {
            let due = config.next_refresh(cached, now);
            lock(&self.refreshing.schedule).reschedule(circle_id, due);
            return None;
        }
        let started = Instant::now();
        let result = self.details.refresh(circle_id, Priority::Background).await;
        let took = started.elapsed();
        let now = utc_now();
        let due = match result {
            Ok(_) => config.next_refresh(self.details.last_updated(circle_id).await, now),
            Err(error) => {
                tracing::warn!(circle_id, %error, "club refresh failed; retrying later");
                now + config.retry
            }
        };
        lock(&self.refreshing.schedule).reschedule(circle_id, due);
        Some(took)
    }
}

pub(crate) fn utc_now() -> DateTime<Utc> {
    SystemTime::now().into()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests;
