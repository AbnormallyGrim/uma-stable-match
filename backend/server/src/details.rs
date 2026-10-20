use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

use moka::future::Cache;
use tokio::sync::mpsc;
use uma_stable_match_scoring::Club;
use uma_stable_match_umamoe_client::{CircleRequest, Client, Error, Priority};

use crate::calibration::club_median;
use crate::pool::ClubPool;

pub type SharedPool = Arc<RwLock<ClubPool>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailConfig {
    pub ttl: Duration,
    pub max_clubs: u64,
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

#[derive(Clone)]
pub struct ClubDetails {
    inner: Arc<Inner>,
}

struct Inner {
    interactive: Client,
    background: Client,
    cache: Cache<u64, Arc<Club>>,
    pool: SharedPool,
    queued: Mutex<HashSet<u64>>,
    sender: mpsc::Sender<u64>,
}

pub struct BackgroundLoader {
    details: ClubDetails,
    receiver: mpsc::Receiver<u64>,
}

impl ClubDetails {
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

    pub async fn get(&self, circle_id: u64, priority: Priority) -> Result<Arc<Club>, Arc<Error>> {
        let inner = &self.inner;
        inner
            .cache
            .try_get_with(circle_id, async {
                let client = match priority {
                    Priority::Interactive => &inner.interactive,
                    Priority::Background => &inner.background,
                };
                let response = client
                    .circle(&CircleRequest::by_circle_id(circle_id))
                    .await?;
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

    pub async fn cached(&self, circle_id: u64) -> Option<Arc<Club>> {
        self.inner.cache.get(&circle_id).await
    }

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

    pub fn queued(&self) -> usize {
        lock(&self.inner.queued).len()
    }

    pub fn cached_clubs(&self) -> u64 {
        self.inner.cache.entry_count()
    }

    pub fn pool(&self) -> &SharedPool {
        &self.inner.pool
    }

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

fn write(pool: &SharedPool) -> std::sync::RwLockWriteGuard<'_, ClubPool> {
    pool.write().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests;
