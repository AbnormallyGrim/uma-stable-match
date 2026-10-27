//! Keeps club details current, refreshing each club after uma.moe's daily update.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Utc};
use tokio::sync::Notify;
use uma_stable_match_umamoe_client::Priority;

use crate::details::ClubDetails;
use crate::pool::Period;
use crate::shared::{RefreshLease, SharedSchedule};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const POLL: Duration = Duration::from_secs(5);
const LEASE_RETRY: Duration = Duration::from_secs(10);

/// When the [`Refresher`] refreshes clubs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshConfig {
    /// Refresh every club this often instead of after uma.moe's daily update.
    pub every: Option<Duration>,
    /// Wait after uma.moe's daily update before refreshing.
    pub margin: Duration,
    /// Wait before checking again a club that uma.moe hasn't updated yet.
    pub recheck: Duration,
    /// Wait before retrying a failed refresh.
    pub retry: Duration,
    /// Joinable clubs ranked up to this refresh at full speed; the rest, and every full or
    /// lower-range club, at half.
    pub fast_ranks: u32,
    /// How much longer lower-range clubs wait between refreshes.
    pub lower_delay: Duration,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            every: None,
            margin: Duration::from_secs(30 * 60),
            recheck: Duration::from_secs(60 * 60),
            retry: Duration::from_secs(5 * 60),
            fast_ranks: 5_000,
            lower_delay: 2 * DAY,
        }
    }
}

impl RefreshConfig {
    /// When to next refresh a club last updated at `last_updated`.
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
}

/// How current the scheduled clubs' details are. The top-level counts are joinable clubs; full
/// and lower-range clubs are in `full` and `lower`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshStatus {
    /// Joinable clubs on the refresh schedule.
    pub scheduled: usize,
    /// Scheduled joinable clubs whose details are stored: in the shared cache with Redis,
    /// otherwise in memory.
    pub loaded: usize,
    /// Scheduled joinable clubs due for a refresh now.
    pub due: usize,
    /// When the longest-waiting due joinable club became due, if any are due.
    pub oldest_due: Option<DateTime<Utc>>,
    /// Time left before the stored details expire, with Redis. `None` without Redis or before any
    /// are stored.
    pub stored_ttl: Option<StoredTtl>,
    /// The full clubs' schedule.
    pub full: TierStatus,
    /// The lower-range clubs' schedule.
    pub lower: TierStatus,
}

/// How current a lower-priority schedule's details are. It's only worked on when no club on a
/// higher-priority schedule is due.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierStatus {
    /// Clubs on the schedule.
    pub scheduled: usize,
    /// Scheduled clubs whose details are stored.
    pub loaded: usize,
    /// Scheduled clubs due for a refresh now.
    pub due: usize,
}

/// Which schedule a club is on, highest priority first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tier {
    Joinable,
    Full,
    Lower,
}

impl Tier {
    const ALL: [Self; 3] = [Self::Joinable, Self::Full, Self::Lower];
}

/// Time to live of the scheduled clubs' details in Redis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredTtl {
    /// The club whose details expire first.
    pub shortest: Duration,
    /// The average across the stored clubs.
    pub average: Duration,
}

pub(crate) struct Refreshing {
    pub(crate) config: RefreshConfig,
    store: Store,
    full: Store,
    lower: Store,
    wake: Notify,
}

enum Store {
    Local(Mutex<Schedule>),
    Shared(SharedSchedule),
}

impl Store {
    fn new(shared: Option<SharedSchedule>) -> Self {
        match shared {
            Some(schedule) => Self::Shared(schedule),
            None => Self::Local(Mutex::new(Schedule::default())),
        }
    }

    async fn replace(
        &self,
        clubs: Vec<(u64, u32)>,
        keep: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) {
        match self {
            Self::Local(schedule) => lock(schedule).replace(clubs, keep, now),
            Self::Shared(schedule) => schedule.replace(clubs, keep, now).await,
        }
    }

    async fn insert_new(&self, circle_id: u64, rank: u32, due: DateTime<Utc>) -> bool {
        match self {
            Self::Local(schedule) => lock(schedule).insert_new(circle_id, rank, due),
            Self::Shared(schedule) => schedule.insert_new(circle_id, rank, due).await,
        }
    }

    async fn due_at(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        match self {
            Self::Local(schedule) => lock(schedule).due_at(circle_id),
            Self::Shared(schedule) => schedule.due_at(circle_id).await,
        }
    }

    async fn len(&self) -> usize {
        match self {
            Self::Local(schedule) => lock(schedule).len(),
            Self::Shared(schedule) => schedule.len().await,
        }
    }

    async fn coverage(
        &self,
        period: Option<Period>,
        loaded: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) -> Option<(usize, usize, usize, Option<StoredTtl>)> {
        match self {
            Self::Local(schedule) => {
                let (scheduled, loaded, due) = lock(schedule).coverage(loaded, now);
                Some((scheduled, loaded, due, None))
            }
            Self::Shared(schedule) => schedule.coverage(period, now).await,
        }
    }

    async fn next(&self) -> Option<(DateTime<Utc>, u32, u64)> {
        match self {
            Self::Local(schedule) => lock(schedule).next(),
            Self::Shared(schedule) => schedule.next().await,
        }
    }

    async fn reschedule(&self, circle_id: u64, due: DateTime<Utc>) {
        match self {
            Self::Local(schedule) => lock(schedule).reschedule(circle_id, due),
            Self::Shared(schedule) => schedule.reschedule(circle_id, due).await,
        }
    }
}

impl Refreshing {
    pub(crate) fn new(config: RefreshConfig, shared: Option<[SharedSchedule; 3]>) -> Self {
        let [joinable, full, lower] = match shared {
            Some(schedules) => schedules.map(Some),
            None => [None, None, None],
        };
        Self {
            config,
            store: Store::new(joinable),
            full: Store::new(full),
            lower: Store::new(lower),
            wake: Notify::new(),
        }
    }

    fn tier(&self, tier: Tier) -> &Store {
        match tier {
            Tier::Joinable => &self.store,
            Tier::Full => &self.full,
            Tier::Lower => &self.lower,
        }
    }

    pub(crate) async fn replace(
        &self,
        clubs: [Vec<(u64, u32)>; 3],
        tier_of: impl Fn(u64) -> Option<Tier>,
        now: DateTime<Utc>,
    ) {
        for (tier, clubs) in Tier::ALL.into_iter().zip(clubs) {
            self.tier(tier)
                .replace(clubs, |id| tier_of(id) == Some(tier), now)
                .await;
        }
        self.wake.notify_one();
    }

    pub(crate) async fn loaded(&self, tier: Tier, circle_id: u64, rank: u32, due: DateTime<Utc>) {
        if self.tier(tier).insert_new(circle_id, rank, due).await {
            self.wake.notify_one();
        }
    }

    pub(crate) async fn due_at(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        for tier in Tier::ALL {
            if let Some(due) = self.tier(tier).due_at(circle_id).await {
                return Some(due);
            }
        }
        None
    }

    pub(crate) async fn len(&self) -> usize {
        let mut total = 0;
        for tier in Tier::ALL {
            total += self.tier(tier).len().await;
        }
        total
    }

    pub(crate) async fn status(
        &self,
        period: Option<Period>,
        loaded: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) -> RefreshStatus {
        let tier_status = async |store: &Store| {
            store
                .coverage(period, &loaded, now)
                .await
                .map(|(scheduled, loaded, due, _)| TierStatus {
                    scheduled,
                    loaded,
                    due,
                })
                .unwrap_or_default()
        };
        let full = tier_status(&self.full).await;
        let lower = tier_status(&self.lower).await;
        let Some((scheduled, loaded, due, stored_ttl)) =
            self.store.coverage(period, &loaded, now).await
        else {
            return RefreshStatus {
                full,
                lower,
                ..RefreshStatus::default()
            };
        };
        let oldest_due = self
            .store
            .next()
            .await
            .map(|(due, _, _)| due)
            .filter(|due| *due <= now);
        RefreshStatus {
            scheduled,
            loaded,
            due,
            oldest_due,
            stored_ttl,
            full,
            lower,
        }
    }

    // The first due club on the highest-priority schedule that has one, else whichever club is due
    // soonest.
    async fn next(&self, now: DateTime<Utc>) -> Option<(DateTime<Utc>, u32, u64, Tier)> {
        let mut soonest: Option<(DateTime<Utc>, u32, u64, Tier)> = None;
        for tier in Tier::ALL {
            let Some((due, rank, id)) = self.tier(tier).next().await else {
                continue;
            };
            if due <= now {
                return Some((due, rank, id, tier));
            }
            if soonest.is_none_or(|(first, ..)| due < first) {
                soonest = Some((due, rank, id, tier));
            }
        }
        soonest
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

    pub(crate) fn coverage(
        &self,
        loaded: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) -> (usize, usize, usize) {
        let present = self.slots.keys().filter(|id| loaded(**id)).count();
        let due = self
            .order
            .iter()
            .take_while(|(due, _, _)| *due <= now)
            .count();
        (self.slots.len(), present, due)
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

/// Refreshes scheduled clubs in the background lane. Get it from [`ClubDetails::refresher`] and
/// run it once with [`Refresher::run`].
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

    /// Refreshes clubs as they come due, for as long as the cache exists. Failures are logged and
    /// retried. With a shared cache, refreshes only while holding the refresher lease.
    pub async fn run(self) {
        loop {
            match &self.refreshing.store {
                Store::Local(_) => self.refresh_while(None).await,
                Store::Shared(schedule) => match schedule.lease().await {
                    Some(lease) => {
                        tracing::info!("running the club refresher");
                        self.refresh_while(Some(&lease)).await;
                        tracing::warn!("lost the refresher lease");
                    }
                    None => tokio::time::sleep(LEASE_RETRY).await,
                },
            }
        }
    }

    async fn refresh_while(&self, lease: Option<&RefreshLease>) {
        let work = async {
            loop {
                self.step().await;
            }
        };
        tokio::pin!(work);
        let Some(lease) = lease else {
            return work.await;
        };
        loop {
            tokio::select! {
                () = &mut work => return,
                () = tokio::time::sleep(RefreshLease::RENEW_EVERY) => {
                    if !lease.renew().await {
                        return;
                    }
                }
            }
        }
    }

    async fn step(&self) {
        let now = utc_now();
        let next = self.refreshing.next(now).await;
        if let Some((_, rank, circle_id, tier)) = next.filter(|(due, ..)| *due <= now) {
            let took = self.refresh(tier, circle_id, now).await;
            let slow = tier != Tier::Joinable || rank > self.refreshing.config.fast_ranks;
            if let Some(took) = took.filter(|_| slow) {
                tokio::time::sleep(took).await;
            }
            return;
        }
        let wait = next.map_or(POLL, |(due, ..)| {
            due.signed_duration_since(now)
                .to_std()
                .unwrap_or_default()
                .min(POLL)
        });
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            () = self.refreshing.wake.notified() => {}
        }
    }

    async fn refresh(&self, tier: Tier, circle_id: u64, now: DateTime<Utc>) -> Option<Duration> {
        let config = self.refreshing.config;
        let store = self.refreshing.tier(tier);
        if let Some(due) = self.details.due(circle_id).await.filter(|due| *due > now) {
            store.reschedule(circle_id, due).await;
            return None;
        }
        let started = Instant::now();
        let result = self.details.refresh(circle_id, Priority::Background).await;
        let took = started.elapsed();
        let now = utc_now();
        let due = match result {
            Ok(_) => self
                .details
                .due(circle_id)
                .await
                .unwrap_or(now + config.recheck),
            Err(error) => {
                tracing::warn!(
                    circle_id,
                    origin = error.origin(),
                    status = ?error.status(),
                    %error,
                    "club refresh failed; retrying later"
                );
                now + config.retry
            }
        };
        store.reschedule(circle_id, due).await;
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
