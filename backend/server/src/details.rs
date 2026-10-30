//! Shared club detail cache and background loader.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Utc};
use moka::future::Cache;
use moka::Expiry;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::field::Empty;
use tracing::{Instrument, Span};
use uma_stable_match_scoring::{Club, MemberSnapshot};
use uma_stable_match_umamoe_client::{Circle, CircleRequest, Priority};

use crate::calibration::club_median;
use crate::pool::{last_month_rate, ClubPool, Period};
use crate::priors::{data_days, data_period, month_label, Priors, CLUB_BLEND_DAYS};
use crate::refresher::{utc_now, RefreshConfig, RefreshStatus, Refresher, Refreshing, Tier};
use crate::scanner::ScanConfig;
use crate::shared::{club_key, SharedCache};
use crate::source::{Source, SourceError};
use crate::traces::in_trace;

/// The club pool, shared between requests and the detail cache.
pub type SharedPool = Arc<RwLock<ClubPool>>;

const KEEP_AFTER_DUE: Duration = Duration::from_secs(24 * 60 * 60);
// Each load renews the whole month's medians, so they outlast any gap between loads.
const KEEP_MEDIANS: Duration = Duration::from_secs(4 * 24 * 60 * 60);

/// Detail cache settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailConfig {
    /// Most bytes of club details kept in the cache, as weighed by [`club_weight`].
    pub max_bytes: u64,
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
            max_bytes: 32 * 1024 * 1024,
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
    source: Source,
    cache: Cache<(u64, Option<Period>), Entry>,
    period: RwLock<Option<Period>>,
    prior_period: RwLock<Option<Period>>,
    priors: Priors,
    pool: SharedPool,
    listed: RwLock<HashMap<u64, Circle>>,
    listing: RwLock<Option<(SystemTime, Option<String>)>>,
    queued: Mutex<HashSet<u64>>,
    sender: mpsc::Sender<u64>,
    shared: SharedCache,
    timing: RefreshConfig,
    scan: ScanConfig,
    refreshing: Option<Arc<Refreshing>>,
}

#[derive(Clone)]
struct Entry {
    club: Arc<Club>,
    last_updated: Option<i64>,
    due_ms: i64,
}

/// Approximate bytes a club's details take in memory.
pub fn club_weight(club: &Club) -> u32 {
    let members: usize = club
        .members
        .iter()
        .map(|m| m.daily_fans.capacity() * size_of::<i64>())
        .sum();
    let bytes = size_of::<Entry>()
        + size_of::<Club>()
        + club.name.capacity()
        + club.comment.as_ref().map_or(0, String::capacity)
        + club.members.capacity() * size_of::<MemberSnapshot>()
        + members;
    u32::try_from(bytes).unwrap_or(u32::MAX)
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
        source: impl Into<Source>,
        pool: SharedPool,
        config: DetailConfig,
    ) -> (Self, BackgroundLoader) {
        Self::with_shared(source, pool, config, SharedCache::none())
    }

    /// Like [`ClubDetails::new`], checking `shared` before the source and storing each loaded club
    /// in it.
    pub fn with_shared(
        source: impl Into<Source>,
        pool: SharedPool,
        config: DetailConfig,
        shared: SharedCache,
    ) -> (Self, BackgroundLoader) {
        let cache = Cache::builder()
            .max_capacity(config.max_bytes)
            .weigher(|_, entry: &Entry| club_weight(&entry.club))
            .expire_after(UntilDue {
                keep: config.refresher && !shared.is_enabled(),
            })
            .build();
        let (sender, receiver) = mpsc::channel(config.queue_size.max(1));
        let details = Self {
            inner: Arc::new(Inner {
                source: source.into(),
                cache,
                period: RwLock::new(None),
                prior_period: RwLock::new(None),
                priors: Priors::default(),
                pool,
                listed: RwLock::new(HashMap::new()),
                listing: RwLock::new(None),
                queued: Mutex::new(HashSet::new()),
                timing: config.refresh,
                scan: config.scan,
                refreshing: config.refresher.then(|| {
                    Arc::new(Refreshing::new(
                        config.refresh,
                        shared.schedules(&config.scan),
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
    pub async fn get(
        &self,
        circle_id: u64,
        priority: Priority,
    ) -> Result<Arc<Club>, Arc<SourceError>> {
        let span = in_trace(|| tracing::info_span!("club details", circle_id, source = Empty));
        self.load(circle_id, priority).instrument(span).await
    }

    async fn load(
        &self,
        circle_id: u64,
        priority: Priority,
    ) -> Result<Arc<Club>, Arc<SourceError>> {
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
                        record_median(inner, &stored.club, circle_id, period).await;
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
                        record_median(inner, &stored.club, circle_id, period).await;
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
    ) -> Result<Arc<Club>, Arc<SourceError>> {
        let inner = &self.inner;
        let period = self.period();
        let entry = fetch(inner, circle_id, period, priority)
            .await
            .map_err(Arc::new)?;
        inner.cache.insert((circle_id, period), entry.clone()).await;
        Ok(entry.club)
    }

    /// When the source last updated a cached club, if it said.
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

    /// How current the scheduled clubs' details are. All zero without a refresher.
    pub async fn refresh_status(&self) -> RefreshStatus {
        let Some(refreshing) = &self.inner.refreshing else {
            return RefreshStatus::default();
        };
        let period = self.period();
        let cache = &self.inner.cache;
        refreshing
            .status(period, |id| cache.contains_key(&(id, period)), utc_now())
            .await
    }

    pub(crate) async fn schedule(&self, clubs: [Vec<(u64, u32)>; 3]) {
        if let Some(refreshing) = &self.inner.refreshing {
            let inner = &self.inner;
            refreshing
                .replace(clubs, |id| tier(inner, id), utc_now())
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

    pub(crate) fn listed_rank(&self, circle_id: u64) -> Option<u32> {
        read(&self.inner.listed)
            .get(&circle_id)
            .and_then(|c| c.monthly_rank)
            .and_then(|r| u32::try_from(r).ok())
    }

    /// A club's details, only if they're cached.
    pub async fn cached(&self, circle_id: u64) -> Option<Arc<Club>> {
        self.inner
            .cache
            .get(&(circle_id, self.period()))
            .await
            .map(|entry| entry.club)
    }

    pub(crate) async fn load_shared(&self, ids: &[u64]) -> Vec<Arc<Club>> {
        let inner = &self.inner;
        let period = self.period();
        let keys: Vec<String> = ids.iter().map(|&id| club_key(id, period)).collect();
        let found = inner.shared.get_many::<Stored>(&keys).await;
        let now = utc_now();
        let mut clubs = Vec::new();
        let mut medians = Vec::new();
        for (&circle_id, stored) in ids.iter().zip(found) {
            let Some(stored) = stored.filter(|s| s.is_current(now)) else {
                continue;
            };
            count_detail("redis");
            if let Some(median) =
                club_median(&stored.club).filter(|_| is_listed_month(inner, &stored.club))
            {
                medians.push((circle_id, median));
            }
            let entry = inner
                .cache
                .entry((circle_id, period))
                .or_insert(Entry::from(stored))
                .await
                .into_value();
            clubs.push(entry.club);
        }
        // One batch, as each update recalibrates and re-sorts the whole pool.
        record_medians(inner, medians, period).await;
        clubs
    }

    /// The month whose details are fetched and served: `None` for the current game month.
    pub fn period(&self) -> Option<Period> {
        *read(&self.inner.period)
    }

    /// Switches to another month's details. Details cached for other months are ignored.
    pub fn set_period(&self, period: Option<Period>) {
        *write(&self.inner.period) = period;
    }

    /// The month whose paces stand in while the listed month has too little data: the previous
    /// game month on its first two days, else `None`.
    pub fn prior_period(&self) -> Option<Period> {
        *read(&self.inner.prior_period)
    }

    pub(crate) fn set_prior_period(&self, period: Option<Period>) {
        *write(&self.inner.prior_period) = period;
    }

    /// Keeps the pool's exact medians as last month's club priors, for a pool about to be
    /// replaced by one for the month after `period`.
    pub(crate) fn keep_club_priors(&self, period: Period) {
        let medians: Vec<(u64, f64)> = read(&self.inner.pool).known_exact_medians().collect();
        self.inner.priors.set_club_medians(period, medians);
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

    /// Approximate bytes of cached club details, as weighed by [`club_weight`].
    pub fn cached_bytes(&self) -> u64 {
        self.inner.cache.weighted_size()
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

    /// Copies the current pool's exact medians into `pool`, such as a freshly scanned one for the
    /// same month. Returns how many were set.
    pub fn carry_exact_medians(&self, pool: &mut ClubPool) -> usize {
        let medians: Vec<(u64, f64)> = read(&self.inner.pool).known_exact_medians().collect();
        pool.set_exact_medians(medians)
    }

    /// Copies the exact medians stored in the shared cache for the listed month into the pool,
    /// for clubs it has none for, such as after a restart. Returns how many were set.
    pub async fn restore_shared_medians(&self) -> usize {
        let Some(month) = listed_month(&self.inner) else {
            return 0;
        };
        let stored = self.inner.shared.medians(&month).await;
        if stored.is_empty() || listed_month(&self.inner).as_ref() != Some(&month) {
            return 0;
        }
        let mut pool = write(&self.inner.pool);
        let missing: Vec<(u64, f64)> = stored
            .into_iter()
            .filter(|(id, _)| pool.contains(*id) && !pool.has_exact_median(*id))
            .collect();
        pool.set_exact_medians(missing)
    }

    /// Copies the exact medians of cached clubs into `pool`, such as a freshly scanned one.
    /// Returns how many were set.
    pub fn restore_exact_medians(&self, pool: &mut ClubPool) -> usize {
        let period = self.period();
        pool.set_exact_medians(
            self.inner
                .cache
                .iter()
                .filter(|(key, entry)| key.1 == period && is_listed_month(&self.inner, &entry.club))
                .filter_map(|(key, entry)| club_median(&entry.club).map(|m| (key.0, m))),
        )
    }
}

impl BackgroundLoader {
    /// Loads queued clubs for as long as the cache exists. Failures are logged.
    pub async fn run(mut self) {
        while let Some(circle_id) = self.receiver.recv().await {
            if let Err(error) = self.details.get(circle_id, Priority::Background).await {
                tracing::warn!(circle_id, origin = error.origin(), %error, "background club load failed");
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
) -> Result<Entry, SourceError> {
    let mut request = CircleRequest::by_circle_id(circle_id);
    if let Some((year, month)) = period {
        request = request.for_month(year, month);
    }
    let mut response = inner
        .source
        .club(&request, priority)
        .await
        .inspect_err(|_| {
            count_detail("error");
        })?;
    count_detail(inner.source.name());
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
    if data_days(&club) < CLUB_BLEND_DAYS {
        blend_in_priors(inner, circle_id, period, &mut club).await;
    }
    record_median(inner, &club, circle_id, period).await;
    let now = utc_now();
    let mut due = inner.timing.next_refresh(
        last_updated.and_then(|t| DateTime::from_timestamp(t, 0)),
        now,
    );
    if inner.scan.is_lower(listed_rank(inner, circle_id)) {
        due += inner.timing.lower_delay;
    }
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

// Early in a month a club's details roll over from last month's: keeps those members' paces as
// priors, then gives this month's members theirs.
async fn blend_in_priors(inner: &Inner, circle_id: u64, period: Option<Period>, club: &mut Club) {
    let previous = match inner.cache.get(&(circle_id, period)).await {
        Some(entry) => Some(entry.club),
        None => inner
            .shared
            .get::<Stored>(&club_key(circle_id, period))
            .await
            .map(|stored| Arc::new(stored.club)),
    };
    if let Some(previous) = previous.filter(|p| data_period(p) < data_period(club)) {
        inner
            .priors
            .capture(&inner.shared, circle_id, &previous)
            .await;
    }
    let fallback = read(&inner.listed).get(&circle_id).and_then(|c| {
        let members = u32::try_from(c.member_count?).ok()?;
        last_month_rate(c.last_month_point?, members, utc_now())
    });
    inner
        .priors
        .attach(&inner.shared, circle_id, club, fallback)
        .await;
}

async fn track(inner: &Inner, circle_id: u64, due: DateTime<Utc>) {
    let Some(refreshing) = &inner.refreshing else {
        return;
    };
    let Some(tier) = tier(inner, circle_id) else {
        return;
    };
    refreshing
        .loaded(tier, circle_id, listed_rank(inner, circle_id), due)
        .await;
}

fn listed_rank(inner: &Inner, circle_id: u64) -> u32 {
    read(&inner.listed)
        .get(&circle_id)
        .and_then(|c| c.monthly_rank)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(u32::MAX)
}

fn tier(inner: &Inner, circle_id: u64) -> Option<Tier> {
    let pool = read(&inner.pool);
    if !pool.contains(circle_id) {
        None
    } else if inner.scan.is_lower(listed_rank(inner, circle_id)) {
        Some(Tier::Lower)
    } else if pool.is_joinable(circle_id) {
        Some(Tier::Joinable)
    } else if pool.contains(circle_id) {
        Some(Tier::Full)
    } else {
        None
    }
}

fn count_detail(source: &'static str) {
    metrics::counter!("club_detail_lookups_total", "source" => source).increment(1);
    Span::current().record("source", source);
}

async fn record_median(inner: &Inner, club: &Club, circle_id: u64, period: Option<Period>) {
    if !is_listed_month(inner, club) {
        return;
    }
    if let Some(median) = club_median(club) {
        record_medians(inner, vec![(circle_id, median)], period).await;
    }
}

// Into the pool and the shared cache, so a restarted instance starts with them.
async fn record_medians(inner: &Inner, medians: Vec<(u64, f64)>, period: Option<Period>) {
    if *read(&inner.period) != period {
        return;
    }
    write(&inner.pool).set_exact_medians(medians.iter().copied());
    if let Some(month) = listed_month(inner) {
        inner
            .shared
            .store_medians(&month, &medians, KEEP_MEDIANS)
            .await;
    }
}

fn listed_month(inner: &Inner) -> Option<String> {
    read(&inner.listing).as_ref()?.1.clone()
}

// Details still from last month don't give this month's pool a median.
fn is_listed_month(inner: &Inner, club: &Club) -> bool {
    match (listed_month(inner), data_period(club)) {
        (Some(listed), Some(period)) => listed == month_label(period),
        _ => true,
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
