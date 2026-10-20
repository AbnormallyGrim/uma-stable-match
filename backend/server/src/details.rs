//! Shared club detail cache and background loader.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

use moka::future::Cache;
use tokio::sync::mpsc;
use uma_stable_match_scoring::Club;
use uma_stable_match_umamoe_client::{Circle, CircleRequest, Client, Error, Priority};

use crate::calibration::club_median;
use crate::pool::ClubPool;

/// The club pool, shared between requests and the detail cache.
pub type SharedPool = Arc<RwLock<ClubPool>>;

/// Detail cache settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailConfig {
    /// How long a club's details are reused.
    pub ttl: Duration,
    /// Most clubs kept in the cache.
    pub max_clubs: u64,
    /// Most clubs waiting for a background load.
    pub queue_size: usize,
}

impl Default for DetailConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(180 * 60),
            max_clubs: 2_000,
            queue_size: 1_000,
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
    cache: Cache<u64, Arc<Club>>,
    pool: SharedPool,
    listed: RwLock<HashMap<u64, Circle>>,
    queued: Mutex<HashSet<u64>>,
    sender: mpsc::Sender<u64>,
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
        let cache = Cache::builder()
            .time_to_live(config.ttl)
            .max_capacity(config.max_clubs)
            .build();
        let (sender, receiver) = mpsc::channel(config.queue_size.max(1));
        let details = Self {
            inner: Arc::new(Inner {
                interactive: client.with_priority(Priority::Interactive),
                background: client.with_priority(Priority::Background),
                cache,
                pool,
                listed: RwLock::new(HashMap::new()),
                queued: Mutex::new(HashSet::new()),
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
    /// club share a single fetch. Failed fetches aren't cached.
    pub async fn get(&self, circle_id: u64, priority: Priority) -> Result<Arc<Club>, Arc<Error>> {
        let inner = &self.inner;
        inner
            .cache
            .try_get_with(circle_id, async {
                let client = match priority {
                    Priority::Interactive => &inner.interactive,
                    Priority::Background => &inner.background,
                };
                let mut response = client
                    .circle(&CircleRequest::by_circle_id(circle_id))
                    .await?;
                if response.circle.circle_id == 0 {
                    if let Some(listed) = read(&inner.listed).get(&circle_id) {
                        response.circle = listed.clone();
                    }
                }
                let mut club = Club::from(response);
                if club.circle_id == 0 {
                    club.circle_id = circle_id;
                }
                if let Some(median) = club_median(&club) {
                    write(&inner.pool).set_exact_median(circle_id, median);
                }
                Ok(Arc::new(club))
            })
            .await
    }

    /// A club's details, only if they're cached.
    pub async fn cached(&self, circle_id: u64) -> Option<Arc<Club>> {
        self.inner.cache.get(&circle_id).await
    }

    /// Queues a club for background loading. Returns `false` if it's cached, already queued, or the
    /// queue is full.
    pub fn queue(&self, circle_id: u64) -> bool {
        if self.inner.cache.contains_key(&circle_id) {
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

    /// Clubs queued or loading in the background.
    pub fn queued(&self) -> usize {
        lock(&self.inner.queued).len()
    }

    /// Approximate number of cached clubs.
    pub fn cached_clubs(&self) -> u64 {
        self.inner.cache.entry_count()
    }

    /// The pool that loads write exact medians into.
    pub fn pool(&self) -> &SharedPool {
        &self.inner.pool
    }

    pub fn replace_pool(&self, pool: ClubPool) {
        *write(&self.inner.pool) = pool;
    }

    pub fn set_listed(&self, rows: impl IntoIterator<Item = (u64, Circle)>) {
        *write(&self.inner.listed) = rows.into_iter().collect();
    }

    pub fn cached_in_pool(&self) -> Vec<Arc<Club>> {
        let pool = read(&self.inner.pool);
        let mut clubs: Vec<Arc<Club>> = self
            .inner
            .cache
            .iter()
            .filter(|(id, _)| pool.contains(**id))
            .map(|(_, club)| club)
            .collect();
        clubs.sort_by_key(|c| (c.monthly_rank.unwrap_or(u32::MAX), c.circle_id));
        clubs
    }

    /// Copies the exact medians of cached clubs into `pool`, such as a freshly scanned one.
    /// Returns how many were set.
    pub fn restore_exact_medians(&self, pool: &mut ClubPool) -> usize {
        pool.set_exact_medians(
            self.inner
                .cache
                .iter()
                .filter_map(|(id, club)| club_median(&club).map(|m| (*id, m))),
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
