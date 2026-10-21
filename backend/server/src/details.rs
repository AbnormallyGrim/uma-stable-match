//! Shared club detail cache and background loader.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Utc};
use moka::future::Cache;
use moka::Expiry;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uma_stable_match_scoring::Club;
use uma_stable_match_umamoe_client::{Circle, CircleRequest, Client, Error, Priority};

use crate::calibration::club_median;
use crate::pool::{ClubPool, Period};
use crate::refresher::{utc_now, RefreshConfig, Refresher, Refreshing};
use crate::scanner::ScanConfig;
use crate::shared::{club_key, SharedCache};

/// The club pool, shared between requests and the detail cache.
pub type SharedPool = Arc<RwLock<ClubPool>>;

const KEEP_AFTER_DUE: Duration = Duration::from_secs(24 * 60 * 60);

/// Detail cache settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailConfig {
    /// Most clubs kept in the cache.
    pub max_clubs: u64,
    /// Most clubs waiting for a background load.
    pub queue_size: usize,
    /// When each club is next refreshed. Details expire from memory then, and stay in the shared
    /// cache a day longer.
    pub refresh: RefreshConfig,
    /// Whether a [`Refresher`] runs. Without a shared cache, details then stay in memory until it
    /// replaces them; with one, the schedule is shared and they expire when due.
    pub refresher: bool,
    /// The scan window whose clubs are refreshed. Names the shared schedule.
    pub scan: ScanConfig,
}

impl Default for DetailConfig {
    fn default() -> Self {
        Self {
            max_clubs: 2_000,
            queue_size: 1_000,
            refresh: RefreshConfig::default(),
            refresher: false,
            scan: ScanConfig::default(),
        }
    }
}

/// Shared, single-flight cache of club details. Clones share one cache and queue.
#[derive(Clone)]
pub struct ClubDetails {
    inner: Arc<Inner>,
}

struct Inner {
    interactive: Client,
    background: Client,
    cache: Cache<(u64, Option<Period>), Entry>,
    period: RwLock<Option<Period>>,
    pool: SharedPool,
    listed: RwLock<HashMap<u64, Circle>>,
    listing: RwLock<Option<(SystemTime, Option<String>)>>,
    queued: Mutex<HashSet<u64>>,
    sender: mpsc::Sender<u64>,
    shared: SharedCache,
    timing: RefreshConfig,
    refreshing: Option<Arc<Refreshing>>,
}

#[derive(Clone)]
struct Entry {
    club: Arc<Club>,
    last_updated: Option<i64>,
    due_ms: i64,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    club: Club,
    last_updated: Option<i64>,
    #[serde(default)]
    due_ms: Option<i64>,
}

impl Stored {
    fn is_current(&self, now: DateTime<Utc>) -> bool {
        self.due_ms.is_some_and(|due| due > now.timestamp_millis())
    }
}

impl From<Stored> for Entry {
    fn from(stored: Stored) -> Self {
        Self {
            club: Arc::new(stored.club),
            last_updated: stored.last_updated,
            due_ms: stored.due_ms.unwrap_or(0),
        }
    }
}

struct UntilDue {
    keep: bool,
}

impl UntilDue {
    fn remaining(&self, entry: &Entry) -> Option<Duration> {
        if self.keep {
            return None;
        }
        let left = entry.due_ms - utc_now().timestamp_millis();
        Some(Duration::from_millis(u64::try_from(left).unwrap_or(0)))
    }
}

impl Expiry<(u64, Option<Period>), Entry> for UntilDue {
    fn expire_after_create(
        &self,
        _: &(u64, Option<Period>),
        entry: &Entry,
        _: Instant,
    ) -> Option<Duration> {
        self.remaining(entry)
    }

    fn expire_after_update(
        &self,
        _: &(u64, Option<Period>),
        entry: &Entry,
        _: Instant,
        _: Option<Duration>,
    ) -> Option<Duration> {
        self.remaining(entry)
    }
}

/// Loads queued clubs in the background lane. Run it once with [`BackgroundLoader::run`].
pub struct BackgroundLoader {
    details: ClubDetails,
    receiver: mpsc::Receiver<u64>,
}

impl std::fmt::Debug for ClubDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClubDetails")
            .field("cached", &self.cached_clubs())
            .field("queued", &self.queued())
            .finish_non_exhaustive()
    }
}

impl ClubDetails {
    /// Builds the cache and the loader for its queue.
    pub fn new(
        client: &Client,
        pool: SharedPool,
        config: DetailConfig,
    ) -> (Self, BackgroundLoader) {
        Self::with_shared(client, pool, config, SharedCache::none())
    }

    /// Like [`ClubDetails::new`], checking `shared` before uma.moe and storing each loaded club in
    /// it.
    pub fn with_shared(
        client: &Client,
        pool: SharedPool,
        config: DetailConfig,
        shared: SharedCache,
    ) -> (Self, BackgroundLoader) {
        let cache = Cache::builder()
            .max_capacity(config.max_clubs)
            .expire_after(UntilDue {
                keep: config.refresher && !shared.is_enabled(),
            })
            .build();
        let (sender, receiver) = mpsc::channel(config.queue_size.max(1));
        let details = Self {
            inner: Arc::new(Inner {
                interactive: client.with_priority(Priority::Interactive),
                background: client.with_priority(Priority::Background),
                cache,
                period: RwLock::new(None),
                pool,
                listed: RwLock::new(HashMap::new()),
                listing: RwLock::new(None),
                queued: Mutex::new(HashSet::new()),
                timing: config.refresh,
                refreshing: config.refresher.then(|| {
                    Arc::new(Refreshing::new(
                        config.refresh,
                        shared.schedule(&config.scan),
                    ))
                }),
                shared,
                sender,
            }),
        };
        let loader = BackgroundLoader {
            details: details.clone(),
            receiver,
        };
        (details, loader)
    }

    /// A club's details, fetched in the given lane if they aren't cached. Concurrent calls for one
    /// club share a single fetch. Failed fetches aren't cached. A shared copy past its next refresh
    /// is fetched again, but still served if that fetch fails.
    pub async fn get(&self, circle_id: u64, priority: Priority) -> Result<Arc<Club>, Arc<Error>> {
        let inner = &self.inner;
        let period = self.period();
        if let Some(entry) = inner.cache.get(&(circle_id, period)).await {
            count_detail("memory");
            return Ok(entry.club);
        }
        inner
            .cache
            .try_get_with((circle_id, period), async {
                let key = club_key(circle_id, period);
                let stale = match inner.shared.get::<Stored>(&key).await {
                    Some(stored) if stored.is_current(utc_now()) => {
                        count_detail("redis");
                        record_median(inner, &stored.club, circle_id, period);
                        return Ok(Entry::from(stored));
                    }
                    stale => stale,
                };
                match fetch(inner, circle_id, period, priority).await {
                    Ok(entry) => Ok(entry),
                    Err(error) => {
                        let Some(mut stored) = stale else {
                            return Err(error);
                        };
                        count_detail("stale");
                        record_median(inner, &stored.club, circle_id, period);
                        stored.due_ms = Some((utc_now() + inner.timing.retry).timestamp_millis());
                        Ok(Entry::from(stored))
                    }
                }
            })
            .await
            .map(|entry| entry.club)
    }

    /// Fetches a club's details in the given lane, even if cached, and replaces the cached copy.
    /// On failure the cached copy stays.
    pub async fn refresh(
        &self,
        circle_id: u64,
        priority: Priority,
    ) -> Result<Arc<Club>, Arc<Error>> {
        let inner = &self.inner;
        let period = self.period();
        let entry = fetch(inner, circle_id, period, priority)
            .await
            .map_err(Arc::new)?;
        inner.cache.insert((circle_id, period), entry.clone()).await;
        Ok(entry.club)
    }

    /// When uma.moe last updated a cached club, if it said.
    pub async fn last_updated(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        let entry = self.inner.cache.get(&(circle_id, self.period())).await?;
        DateTime::from_timestamp(entry.last_updated?, 0)
    }

    /// The refresher for this cache, if it has one. Run one per instance; with a shared cache, only
    /// the instance holding the refresher lease refreshes.
    pub fn refresher(&self) -> Option<Refresher> {
        let refreshing = self.inner.refreshing.clone()?;
        Some(Refresher::new(self.clone(), refreshing))
    }

    /// Whether this cache has a refresher.
    pub fn refreshes(&self) -> bool {
        self.inner.refreshing.is_some()
    }

    /// When a scheduled club is next refreshed.
    pub async fn next_refresh(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        self.inner.refreshing.as_ref()?.due_at(circle_id).await
    }

    /// Clubs on the refresh schedule.
    pub async fn scheduled(&self) -> usize {
        match &self.inner.refreshing {
            Some(refreshing) => refreshing.len().await,
            None => 0,
        }
    }

    pub(crate) async fn schedule(&self, clubs: Vec<(u64, u32)>) {
        if let Some(refreshing) = &self.inner.refreshing {
            let pool = &self.inner.pool;
            refreshing
                .replace(clubs, |id| read(pool).contains(id), utc_now())
                .await;
        }
    }

    pub(crate) async fn due(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        let period = self.period();
        let due_ms = match self.inner.cache.get(&(circle_id, period)).await {
            Some(entry) => entry.due_ms,
            None => {
                let key = club_key(circle_id, period);
                self.inner.shared.get::<Stored>(&key).await?.due_ms?
            }
        };
        DateTime::from_timestamp_millis(due_ms)
    }

    /// A club's details, only if they're cached.
    pub async fn cached(&self, circle_id: u64) -> Option<Arc<Club>> {
        self.inner
            .cache
            .get(&(circle_id, self.period()))
            .await
            .map(|entry| entry.club)
    }

    /// The month whose details are fetched and served: `None` for the current game month.
    pub fn period(&self) -> Option<Period> {
        *read(&self.inner.period)
    }

    /// Switches to another month's details. Details cached for other months are ignored.
    pub fn set_period(&self, period: Option<Period>) {
        *write(&self.inner.period) = period;
    }

    /// Queues a club for background loading. Returns `false` if it's cached, already queued, or the
    /// queue is full.
    pub fn queue(&self, circle_id: u64) -> bool {
        if self.inner.cache.contains_key(&(circle_id, self.period())) {
            return false;
        }
        let mut queued = lock(&self.inner.queued);
        if !queued.insert(circle_id) {
            return false;
        }
        if self.inner.sender.try_send(circle_id).is_err() {
            queued.remove(&circle_id);
            return false;
        }
        true
    }

    pub(crate) async fn settle(&self) {
        self.inner.cache.run_pending_tasks().await;
    }

    /// Clubs queued or loading in the background.
    pub fn queued(&self) -> usize {
        lock(&self.inner.queued).len()
    }

    /// Approximate number of cached clubs.
    pub fn cached_clubs(&self) -> u64 {
        self.inner.cache.entry_count()
    }

    pub(crate) fn shared(&self) -> &SharedCache {
        &self.inner.shared
    }

    /// The pool that loads write exact medians into.
    pub fn pool(&self) -> &SharedPool {
        &self.inner.pool
    }

    /// Replaces the shared pool, such as after a scan.
    pub fn replace_pool(&self, pool: ClubPool) {
        *write(&self.inner.pool) = pool;
    }

    pub(crate) fn listing(&self) -> Option<(SystemTime, Option<String>)> {
        read(&self.inner.listing).clone()
    }

    pub(crate) fn set_listing(&self, at: SystemTime, month: Option<String>) {
        *write(&self.inner.listing) = Some((at, month));
    }

    /// Replaces the listed rows used when a club's details come back without club information.
    pub fn set_listed(&self, rows: impl IntoIterator<Item = (u64, Circle)>) {
        *write(&self.inner.listed) = rows.into_iter().collect();
    }

    /// Cached clubs that are still in the pool, best rank first.
    pub fn cached_in_pool(&self) -> Vec<Arc<Club>> {
        let period = self.period();
        let pool = read(&self.inner.pool);
        let mut clubs: Vec<Arc<Club>> = self
            .inner
            .cache
            .iter()
            .filter(|(key, _)| key.1 == period && pool.contains(key.0))
            .map(|(_, entry)| entry.club)
            .collect();
        clubs.sort_by_key(|c| (c.monthly_rank.unwrap_or(u32::MAX), c.circle_id));
        clubs
    }

    /// Copies the exact medians of cached clubs into `pool`, such as a freshly scanned one.
    /// Returns how many were set.
    pub fn restore_exact_medians(&self, pool: &mut ClubPool) -> usize {
        let period = self.period();
        pool.set_exact_medians(
            self.inner
                .cache
                .iter()
                .filter(|(key, _)| key.1 == period)
                .filter_map(|(key, entry)| club_median(&entry.club).map(|m| (key.0, m))),
        )
    }
}

impl BackgroundLoader {
    /// Loads queued clubs for as long as the cache exists. Failures are logged.
    pub async fn run(mut self) {
        while let Some(circle_id) = self.receiver.recv().await {
            if let Err(error) = self.details.get(circle_id, Priority::Background).await {
                tracing::warn!(circle_id, %error, "background club load failed");
            }
            lock(&self.details.inner.queued).remove(&circle_id);
        }
    }
}

async fn fetch(
    inner: &Inner,
    circle_id: u64,
    period: Option<Period>,
    priority: Priority,
) -> Result<Entry, Error> {
    let client = match priority {
        Priority::Interactive => &inner.interactive,
        Priority::Background => &inner.background,
    };
    let mut request = CircleRequest::by_circle_id(circle_id);
    if let Some((year, month)) = period {
        request = request.for_month(year, month);
    }
    let mut response = client.circle(&request).await.inspect_err(|_| {
        count_detail("error");
    })?;
    count_detail("umamoe");
    if response.circle.circle_id == 0 {
        if let Some(listed) = read(&inner.listed).get(&circle_id) {
            response.circle = listed.clone();
        }
    }
    let last_updated = response
        .circle
        .last_updated
        .as_deref()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.timestamp());
    let mut club = Club::from(response);
    if club.circle_id == 0 {
        club.circle_id = circle_id;
    }
    record_median(inner, &club, circle_id, period);
    let now = utc_now();
    let due = inner.timing.next_refresh(
        last_updated.and_then(|t| DateTime::from_timestamp(t, 0)),
        now,
    );
    track(inner, circle_id, due).await;
    let keep = due.signed_duration_since(now).to_std().unwrap_or_default() + KEEP_AFTER_DUE;
    let stored = Stored {
        club,
        last_updated,
        due_ms: Some(due.timestamp_millis()),
    };
    inner
        .shared
        .set(&club_key(circle_id, period), &stored, keep)
        .await;
    Ok(Entry::from(stored))
}

async fn track(inner: &Inner, circle_id: u64, due: DateTime<Utc>) {
    let Some(refreshing) = &inner.refreshing else {
        return;
    };
    if !read(&inner.pool).contains(circle_id) {
        return;
    }
    let rank = read(&inner.listed)
        .get(&circle_id)
        .and_then(|c| c.monthly_rank)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(u32::MAX);
    refreshing.loaded(circle_id, rank, due).await;
}

fn count_detail(source: &'static str) {
    metrics::counter!("club_detail_lookups_total", "source" => source).increment(1);
}

fn record_median(inner: &Inner, club: &Club, circle_id: u64, period: Option<Period>) {
    if let Some(median) = club_median(club) {
        if *read(&inner.period) == period {
            write(&inner.pool).set_exact_median(circle_id, median);
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests;
