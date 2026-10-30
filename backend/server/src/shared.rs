//! Optional cache shared between server instances.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Cmd, FromRedisValue, Script, ScriptInvocation};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Sha256;
use tokio::time::timeout;
use tracing::{Instrument, Span};

use crate::pool::Period;
use crate::refresher::StoredTtl;
use crate::scanner::ScanConfig;
use crate::traces::in_trace;

const KEY_PREFIX: &str = "usm:v1";
const MIN_SECRET_LEN: usize = 32;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OP_TIMEOUT: Duration = Duration::from_millis(500);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
const SCAN_LEASE: Duration = Duration::from_secs(60);
const RANK_SLOTS: u64 = 1 << 21;

/// A `redis://` or `rediss://` URL. `Debug` output hides it, since it can hold a password.
#[derive(Clone, PartialEq, Eq)]
pub struct RedisUrl(String);

impl RedisUrl {
    /// The URL, or `None` without a `redis://` or `rediss://` scheme and a host.
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value
            .strip_prefix("redis://")
            .or_else(|| value.strip_prefix("rediss://"))?;
        (!rest.is_empty()).then(|| Self(value.to_owned()))
    }

    /// The URL as given, credentials included.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RedisUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RedisUrl(<redacted>)")
    }
}

/// Secret that keys the trainer ID hash in shared-cache keys. `Debug` output hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct KeySecret(String);

impl KeySecret {
    /// The secret, or `None` if it's shorter than 32 characters.
    pub fn parse(value: &str) -> Option<Self> {
        (value.chars().count() >= MIN_SECRET_LEN).then(|| Self(value.to_owned()))
    }
}

impl fmt::Debug for KeySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeySecret(<redacted>)")
    }
}

/// Why the shared cache couldn't start. Messages never include the URL.
#[derive(Debug, thiserror::Error)]
pub enum SharedCacheError {
    /// Redis couldn't be reached or didn't answer `PING`, with the kind of failure.
    #[error("could not connect to Redis ({0})")]
    Connect(String),
}

/// A cache shared between server instances, behind each instance's own cache. The default
/// stores nothing.
#[derive(Debug, Clone, Default)]
pub struct SharedCache {
    backend: Backend,
}

#[derive(Debug, Clone, Default)]
enum Backend {
    #[default]
    None,
    Redis(RedisBackend),
}

#[derive(Clone)]
struct RedisBackend {
    conn: ConnectionManager,
    secret: Arc<KeySecret>,
    healthy: Arc<AtomicBool>,
}

impl fmt::Debug for RedisBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisBackend").finish_non_exhaustive()
    }
}

impl RedisBackend {
    fn up(&self) {
        metrics::gauge!("redis_up").set(1.0);
        if !self.healthy.swap(true, Ordering::Relaxed) {
            tracing::info!("Redis is available again");
        }
    }

    fn down(&self, reason: &str) {
        metrics::gauge!("redis_up").set(0.0);
        metrics::counter!("redis_errors_total", "reason" => reason.to_owned()).increment(1);
        if self.healthy.swap(false, Ordering::Relaxed) {
            tracing::warn!(reason, "Redis is unavailable; using the local cache only");
        }
    }

    async fn invoke<T: FromRedisValue>(
        &self,
        invocation: &ScriptInvocation<'_>,
        limit: Duration,
    ) -> Option<T> {
        let mut conn = self.conn.clone();
        let started = Instant::now();
        let reply = timeout(limit, invocation.invoke_async(&mut conn))
            .instrument(redis_span("script"))
            .await;
        observe("script", started);
        match reply {
            Ok(Ok(value)) => {
                self.up();
                Some(value)
            }
            Ok(Err(error)) => {
                self.down(&kind(&error));
                None
            }
            Err(_) => {
                self.down("timed out");
                None
            }
        }
    }

    async fn query<T: FromRedisValue>(&self, cmd: &Cmd, limit: Duration) -> Option<T> {
        self.query_as("schedule", cmd, limit).await
    }

    async fn query_as<T: FromRedisValue>(
        &self,
        op: &'static str,
        cmd: &Cmd,
        limit: Duration,
    ) -> Option<T> {
        let mut conn = self.conn.clone();
        let started = Instant::now();
        let reply = timeout(limit, cmd.query_async(&mut conn))
            .instrument(redis_span(op))
            .await;
        observe(op, started);
        match reply {
            Ok(Ok(value)) => {
                self.up();
                Some(value)
            }
            Ok(Err(error)) => {
                self.down(&kind(&error));
                None
            }
            Err(_) => {
                self.down("timed out");
                None
            }
        }
    }
}

impl SharedCache {
    /// A shared cache that stores nothing.
    pub fn none() -> Self {
        Self::default()
    }

    /// Connects to Redis and checks it answers, within 5 seconds. Later failures don't end the
    /// connection: calls count as misses while it reconnects.
    pub async fn connect(url: &RedisUrl, secret: KeySecret) -> Result<Self, SharedCacheError> {
        let client =
            redis::Client::open(url.expose()).map_err(|e| SharedCacheError::Connect(kind(&e)))?;
        let mut conn = timeout(CONNECT_TIMEOUT, ConnectionManager::new(client))
            .await
            .map_err(|_| SharedCacheError::Connect("timed out".into()))?
            .map_err(|e| SharedCacheError::Connect(kind(&e)))?;
        timeout(
            CONNECT_TIMEOUT,
            redis::cmd("PING").query_async::<String>(&mut conn),
        )
        .await
        .map_err(|_| SharedCacheError::Connect("timed out".into()))?
        .map_err(|e| SharedCacheError::Connect(kind(&e)))?;
        metrics::gauge!("redis_up").set(1.0);
        Ok(Self {
            backend: Backend::Redis(RedisBackend {
                conn,
                secret: Arc::new(secret),
                healthy: Arc::new(AtomicBool::new(true)),
            }),
        })
    }

    /// Whether values are actually shared.
    pub fn is_enabled(&self) -> bool {
        match self.backend {
            Backend::None => false,
            Backend::Redis(_) => true,
        }
    }

    /// Shared-cache key for a trainer's pace in a month. With Redis, the trainer ID is replaced by
    /// a keyed hash, so Redis never holds it.
    pub fn pace_key(&self, viewer_id: u64, period: Option<Period>) -> String {
        let id = match &self.backend {
            Backend::None => viewer_id.to_string(),
            Backend::Redis(redis) => trainer_digest(&redis.secret, viewer_id),
        };
        format!("{KEY_PREFIX}:pace:{id}:{}", period_part(period))
    }

    /// The stored value, or `None` when it's missing, expired or unreadable.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_within(key, OP_TIMEOUT).await
    }

    async fn get_within<T: DeserializeOwned>(&self, key: &str, limit: Duration) -> Option<T> {
        let Backend::Redis(redis) = &self.backend else {
            return None;
        };
        let mut conn = redis.conn.clone();
        let started = Instant::now();
        let reply = timeout(limit, conn.get::<_, Option<Vec<u8>>>(key))
            .instrument(redis_span("get"))
            .await;
        observe("get", started);
        match reply {
            Ok(Ok(value)) => {
                redis.up();
                serde_json::from_slice(&value?).ok()
            }
            Ok(Err(error)) => {
                redis.down(&kind(&error));
                None
            }
            Err(_) => {
                redis.down("timed out");
                None
            }
        }
    }

    /// The stored values for `keys`, in order, read in one round trip. Misses are `None`, as with
    /// [`SharedCache::get`].
    pub async fn get_many<T: DeserializeOwned>(&self, keys: &[String]) -> Vec<Option<T>> {
        let misses = || keys.iter().map(|_| None).collect();
        let Backend::Redis(redis) = &self.backend else {
            return misses();
        };
        if keys.is_empty() {
            return Vec::new();
        }
        let mut mget = redis::cmd("MGET");
        mget.arg(keys);
        let mut conn = redis.conn.clone();
        let started = Instant::now();
        let span = redis_span("mget");
        span.record("db.operation.batch.size", keys.len());
        let reply = timeout(
            OP_TIMEOUT,
            mget.query_async::<Vec<Option<Vec<u8>>>>(&mut conn),
        )
        .instrument(span)
        .await;
        observe("mget", started);
        match reply {
            Ok(Ok(values)) if values.len() == keys.len() => {
                redis.up();
                values
                    .into_iter()
                    .map(|value| serde_json::from_slice(&value?).ok())
                    .collect()
            }
            Ok(Ok(_)) => {
                redis.up();
                misses()
            }
            Ok(Err(error)) => {
                redis.down(&kind(&error));
                misses()
            }
            Err(_) => {
                redis.down("timed out");
                misses()
            }
        }
    }

    /// Redis's key, TTL, memory and eviction figures from one `INFO` call, or `None` without Redis
    /// or if it doesn't answer.
    pub(crate) async fn stats(&self) -> Option<RedisStats> {
        let Backend::Redis(redis) = &self.backend else {
            return None;
        };
        let mut info = redis::cmd("INFO");
        info.arg("memory").arg("stats").arg("keyspace");
        let mut conn = redis.conn.clone();
        let started = Instant::now();
        let reply = timeout(OP_TIMEOUT, info.query_async::<String>(&mut conn)).await;
        observe("info", started);
        match reply {
            Ok(Ok(text)) => {
                redis.up();
                Some(RedisStats::parse(&text))
            }
            Ok(Err(error)) => {
                redis.down(&kind(&error));
                None
            }
            Err(_) => {
                redis.down("timed out");
                None
            }
        }
    }

    /// Adds one to the search total, which never expires. Failures are ignored.
    pub async fn count_search(&self) {
        let Backend::Redis(redis) = &self.backend else {
            return;
        };
        let mut incr = redis::cmd("INCR");
        incr.arg(searches_key());
        redis.query_as::<i64>("incr", &incr, OP_TIMEOUT).await;
    }

    /// The search total, `0` before the first search, or `None` without Redis or if it doesn't
    /// answer.
    pub async fn searches(&self) -> Option<u64> {
        let Backend::Redis(redis) = &self.backend else {
            return None;
        };
        let mut get = redis::cmd("GET");
        get.arg(searches_key());
        redis
            .query_as::<Option<u64>>("get", &get, OP_TIMEOUT)
            .await
            .map(Option::unwrap_or_default)
    }

    /// Stores a value until `ttl` passes. Failures are ignored, as the value can be fetched again.
    pub async fn set<T: Serialize + Sync>(&self, key: &str, value: &T, ttl: Duration) {
        let Backend::Redis(redis) = &self.backend else {
            return;
        };
        let Ok(bytes) = serde_json::to_vec(value) else {
            return;
        };
        let mut conn = redis.conn.clone();
        let started = Instant::now();
        let reply = timeout(
            OP_TIMEOUT,
            conn.pset_ex::<_, _, ()>(key, bytes, millis(ttl)),
        )
        .instrument(redis_span("set"))
        .await;
        observe("set", started);
        match reply {
            Ok(Ok(())) => redis.up(),
            Ok(Err(error)) => redis.down(&kind(&error)),
            Err(_) => redis.down("timed out"),
        }
    }

    /// Records clubs' exact medians for a game month (`YYYY-MM`), keeping them for `ttl` after the
    /// last write. Failures are ignored.
    pub(crate) async fn store_medians(&self, month: &str, medians: &[(u64, f64)], ttl: Duration) {
        self.store_hash(&medians_key(month), medians, ttl).await;
    }

    /// Records trainers' paces in a game month (`YYYY-MM`), as priors for the next month, keeping
    /// them for `ttl` after the last write. Failures are ignored.
    pub(crate) async fn store_priors(&self, month: &str, paces: &[(u64, f64)], ttl: Duration) {
        self.store_hash(&priors_key(month), paces, ttl).await;
    }

    /// The paces stored by [`SharedCache::store_priors`] for these trainers, in order.
    pub(crate) async fn priors(&self, month: &str, viewer_ids: &[u64]) -> Vec<Option<f64>> {
        self.hash_values(&priors_key(month), viewer_ids).await
    }

    /// The exact medians stored for these clubs in a game month (`YYYY-MM`), in order.
    pub(crate) async fn medians_of(&self, month: &str, circle_ids: &[u64]) -> Vec<Option<f64>> {
        self.hash_values(&medians_key(month), circle_ids).await
    }

    async fn store_hash(&self, key: &str, values: &[(u64, f64)], ttl: Duration) {
        let Backend::Redis(redis) = &self.backend else {
            return;
        };
        if values.is_empty() {
            return;
        }
        let script = Script::new(MEDIANS_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation.key(key).arg(millis(ttl));
        for (id, value) in values {
            invocation.arg(id).arg(value);
        }
        redis.invoke::<i64>(&invocation, OP_TIMEOUT).await;
    }

    async fn hash_values(&self, key: &str, ids: &[u64]) -> Vec<Option<f64>> {
        let misses = || ids.iter().map(|_| None).collect();
        let Backend::Redis(redis) = &self.backend else {
            return misses();
        };
        if ids.is_empty() {
            return Vec::new();
        }
        let mut hmget = redis::cmd("HMGET");
        hmget.arg(key).arg(ids);
        match redis
            .query_as::<Vec<Option<String>>>("hmget", &hmget, OP_TIMEOUT)
            .await
        {
            Some(values) if values.len() == ids.len() => values
                .into_iter()
                .map(|value| value?.parse().ok())
                .collect(),
            _ => misses(),
        }
    }

    /// The exact medians stored for a game month (`YYYY-MM`), or none without Redis or if it
    /// doesn't answer.
    pub(crate) async fn medians(&self, month: &str) -> Vec<(u64, f64)> {
        let Backend::Redis(redis) = &self.backend else {
            return Vec::new();
        };
        let mut all = redis::cmd("HGETALL");
        all.arg(medians_key(month));
        redis
            .query_as::<Vec<(String, String)>>("hgetall", &all, SNAPSHOT_TIMEOUT)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, median)| Some((id.parse().ok()?, median.parse().ok()?)))
            .collect()
    }

    /// Decides in one Redis script whether this instance scans, waits, or loads a published scan.
    pub(crate) async fn scan_turn(&self, config: &ScanConfig) -> ScanTurn {
        let Backend::Redis(redis) = &self.backend else {
            return ScanTurn::Alone;
        };
        let keys = ScanKeys::new(config);
        let token = format!("{:016x}{:016x}", fastrand::u64(..), fastrand::u64(..));
        let script = Script::new(TURN_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&keys.fresh)
            .key(&keys.lock)
            .key(&keys.progress_id)
            .arg(&token)
            .arg(millis(SCAN_LEASE));
        let Some(reply) = redis.invoke::<Vec<String>>(&invocation, OP_TIMEOUT).await else {
            return ScanTurn::Alone;
        };
        match reply.as_slice() {
            [state, progress] if state == "lead" => ScanTurn::Lead {
                lease: ScanLease {
                    redis: redis.clone(),
                    keys,
                    token,
                },
                progress: non_empty(progress),
            },
            [state, ttl, id] if state == "fresh" => ScanTurn::Fresh {
                id: id.clone(),
                remaining: Duration::from_millis(ttl.parse().unwrap_or(0)),
            },
            [state, progress] if state == "wait" => ScanTurn::Wait {
                progress: non_empty(progress),
            },
            _ => ScanTurn::Wait { progress: None },
        }
    }

    pub(crate) fn schedules(&self, config: &ScanConfig) -> Option<[SharedSchedule; 3]> {
        let Backend::Redis(redis) = &self.backend else {
            return None;
        };
        let keys = ScanKeys::new(config);
        let schedule = |key: String| SharedSchedule {
            redis: redis.clone(),
            key,
            lease: keys.refresher.clone(),
        };
        let full = format!("{}-full", keys.schedule);
        let lower = format!("{}-lower", keys.schedule);
        Some([
            schedule(keys.schedule.clone()),
            schedule(full),
            schedule(lower),
        ])
    }

    /// The club list saved by the scan in progress for this scan window.
    pub(crate) async fn scan_in_progress<T: DeserializeOwned>(
        &self,
        config: &ScanConfig,
    ) -> Option<T> {
        self.get_within(&ScanKeys::new(config).progress, SNAPSHOT_TIMEOUT)
            .await
    }

    pub(crate) async fn published_scan<T: DeserializeOwned>(
        &self,
        config: &ScanConfig,
    ) -> Option<T> {
        self.get_within(&ScanKeys::new(config).latest, SNAPSHOT_TIMEOUT)
            .await
    }
}

/// What an instance does when a scan is due.
pub(crate) enum ScanTurn {
    /// No Redis, or it didn't answer: scan without sharing.
    Alone,
    /// This instance holds the scan lock: scan, or resume the saved scan named by `progress`, then
    /// publish.
    Lead {
        lease: ScanLease,
        progress: Option<String>,
    },
    /// Another instance is scanning: check again shortly. `progress` names its saved club list
    /// once it has one.
    Wait { progress: Option<String> },
    /// A scan was published less than an interval ago: load it if it's new.
    Fresh { id: String, remaining: Duration },
}

/// The scan lock, held until published or released, or until it lapses without renewal.
pub(crate) struct ScanLease {
    redis: RedisBackend,
    keys: ScanKeys,
    token: String,
}

impl ScanLease {
    pub(crate) const RENEW_EVERY: Duration = Duration::from_secs(SCAN_LEASE.as_secs() / 3);

    pub(crate) fn id(&self) -> &str {
        &self.token
    }

    pub(crate) async fn renew(&self) {
        let script = Script::new(RENEW_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.keys.lock)
            .arg(&self.token)
            .arg(millis(SCAN_LEASE));
        if self.redis.invoke::<i64>(&invocation, OP_TIMEOUT).await == Some(0) {
            tracing::warn!("lost the scan lease; another instance may scan too");
        }
    }

    /// Saves the scan's club list so a restarted or replacement instance can resume it, for
    /// `keep_for`. Does nothing once another instance holds the lock.
    pub(crate) async fn save_progress<T: Serialize>(&self, value: &T, keep_for: Duration) {
        let Ok(bytes) = serde_json::to_vec(value) else {
            return;
        };
        let script = Script::new(PROGRESS_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.keys.lock)
            .key(&self.keys.progress)
            .key(&self.keys.progress_id)
            .arg(&self.token)
            .arg(bytes)
            .arg(millis(keep_for));
        self.redis
            .invoke::<i64>(&invocation, SNAPSHOT_TIMEOUT)
            .await;
    }

    /// Stores `value` for two intervals, marks it current for one, clears the saved scan in
    /// progress, and releases the lock.
    pub(crate) async fn publish<T: Serialize>(self, value: &T, fresh_for: Duration) {
        let Ok(bytes) = serde_json::to_vec(value) else {
            self.release().await;
            return;
        };
        let script = Script::new(PUBLISH_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.keys.latest)
            .key(&self.keys.fresh)
            .key(&self.keys.lock)
            .key(&self.keys.progress)
            .key(&self.keys.progress_id)
            .arg(bytes)
            .arg(millis(fresh_for * 2))
            .arg(&self.token)
            .arg(millis(fresh_for));
        self.redis
            .invoke::<i64>(&invocation, SNAPSHOT_TIMEOUT)
            .await;
    }

    pub(crate) async fn release(self) {
        let script = Script::new(RELEASE_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation.key(&self.keys.lock).arg(&self.token);
        self.redis.invoke::<i64>(&invocation, OP_TIMEOUT).await;
    }
}

pub(crate) struct SharedSchedule {
    redis: RedisBackend,
    key: String,
    lease: String,
}

impl SharedSchedule {
    pub(crate) async fn replace(
        &self,
        clubs: Vec<(u64, u32)>,
        keep: impl Fn(u64) -> bool,
        now: DateTime<Utc>,
    ) {
        let mut members = redis::cmd("ZRANGE");
        members.arg(&self.key).arg(0).arg(-1);
        let Some(members) = self
            .redis
            .query::<Vec<String>>(&members, SNAPSHOT_TIMEOUT)
            .await
        else {
            return;
        };
        let dropped: Vec<&String> = members
            .iter()
            .filter(|m| m.parse().map_or(true, |id| !keep(id)))
            .collect();
        if !dropped.is_empty() {
            let mut remove = redis::cmd("ZREM");
            remove.arg(&self.key).arg(dropped);
            self.redis.query::<i64>(&remove, SNAPSHOT_TIMEOUT).await;
        }
        if clubs.is_empty() {
            return;
        }
        let script = Script::new(SCHEDULE_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.key)
            .arg(now.timestamp().max(0))
            .arg(RANK_SLOTS);
        for (circle_id, rank) in clubs {
            invocation.arg(circle_id).arg(rank_slot(rank));
        }
        self.redis
            .invoke::<i64>(&invocation, SNAPSHOT_TIMEOUT)
            .await;
    }

    pub(crate) async fn insert_new(&self, circle_id: u64, rank: u32, due: DateTime<Utc>) -> bool {
        let mut add = redis::cmd("ZADD");
        add.arg(&self.key)
            .arg("NX")
            .arg(score(due, rank))
            .arg(circle_id);
        self.redis.query::<i64>(&add, OP_TIMEOUT).await == Some(1)
    }

    pub(crate) async fn reschedule(&self, circle_id: u64, due: DateTime<Utc>) {
        let script = Script::new(RESCHEDULE_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.key)
            .arg(circle_id)
            .arg(due_seconds(due))
            .arg(RANK_SLOTS);
        self.redis.invoke::<i64>(&invocation, OP_TIMEOUT).await;
    }

    pub(crate) async fn next(&self) -> Option<(DateTime<Utc>, u32, u64)> {
        let mut first = redis::cmd("ZRANGE");
        first.arg(&self.key).arg(0).arg(0).arg("WITHSCORES");
        let reply = self
            .redis
            .query::<Vec<(String, f64)>>(&first, OP_TIMEOUT)
            .await?;
        let (member, score) = reply.into_iter().next()?;
        let (due, rank) = decode(score);
        Some((due, rank, member.parse().ok()?))
    }

    pub(crate) async fn due_at(&self, circle_id: u64) -> Option<DateTime<Utc>> {
        let mut get = redis::cmd("ZSCORE");
        get.arg(&self.key).arg(circle_id);
        let score = self.redis.query::<Option<f64>>(&get, OP_TIMEOUT).await??;
        Some(decode(score).0)
    }

    pub(crate) async fn len(&self) -> usize {
        let mut count = redis::cmd("ZCARD");
        count.arg(&self.key);
        self.redis
            .query::<usize>(&count, OP_TIMEOUT)
            .await
            .unwrap_or(0)
    }

    pub(crate) async fn coverage(
        &self,
        period: Option<Period>,
        now: DateTime<Utc>,
    ) -> Option<(usize, usize, usize, Option<StoredTtl>)> {
        let script = Script::new(COVERAGE_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.key)
            .arg(format!("{KEY_PREFIX}:club:"))
            .arg(format!(":{}", period_part(period)))
            .arg(due_by(now));
        let reply = self
            .redis
            .invoke::<Vec<i64>>(&invocation, SNAPSHOT_TIMEOUT)
            .await?;
        let count = |n: i64| usize::try_from(n).unwrap_or(0);
        let ms = |n: i64| Duration::from_millis(u64::try_from(n).unwrap_or(0));
        match reply.as_slice() {
            &[scheduled, loaded, due, expiring, shortest, total] => {
                let ttl = (expiring > 0).then(|| StoredTtl {
                    shortest: ms(shortest),
                    average: ms(total / expiring),
                });
                Some((count(scheduled), count(loaded), count(due), ttl))
            }
            _ => None,
        }
    }

    pub(crate) async fn lease(&self) -> Option<RefreshLease> {
        let token = format!("{:016x}{:016x}", fastrand::u64(..), fastrand::u64(..));
        let mut take = redis::cmd("SET");
        take.arg(&self.lease)
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(millis(SCAN_LEASE));
        self.redis
            .query::<Option<String>>(&take, OP_TIMEOUT)
            .await??;
        Some(RefreshLease {
            redis: self.redis.clone(),
            key: self.lease.clone(),
            token,
        })
    }
}

pub(crate) struct RefreshLease {
    redis: RedisBackend,
    key: String,
    token: String,
}

impl RefreshLease {
    pub(crate) const RENEW_EVERY: Duration = ScanLease::RENEW_EVERY;

    pub(crate) async fn renew(&self) -> bool {
        let script = Script::new(RENEW_SCRIPT);
        let mut invocation = script.prepare_invoke();
        invocation
            .key(&self.key)
            .arg(&self.token)
            .arg(millis(SCAN_LEASE));
        self.redis.invoke::<i64>(&invocation, OP_TIMEOUT).await == Some(1)
    }
}

fn due_seconds(due: DateTime<Utc>) -> u64 {
    u64::try_from(due.timestamp_millis().max(0))
        .unwrap_or(0)
        .div_ceil(1000)
}

fn due_by(now: DateTime<Utc>) -> u64 {
    let seconds = u64::try_from(now.timestamp().max(0)).unwrap_or(0);
    seconds * RANK_SLOTS + RANK_SLOTS - 1
}

fn rank_slot(rank: u32) -> u64 {
    u64::from(rank).min(RANK_SLOTS - 1)
}

pub(crate) fn score(due: DateTime<Utc>, rank: u32) -> u64 {
    due_seconds(due) * RANK_SLOTS + rank_slot(rank)
}

pub(crate) fn decode(score: f64) -> (DateTime<Utc>, u32) {
    let score = score as u64;
    let due = i64::try_from(score / RANK_SLOTS).unwrap_or(i64::MAX);
    let rank = u32::try_from(score % RANK_SLOTS).unwrap_or(u32::MAX);
    let rank = if u64::from(rank) == RANK_SLOTS - 1 {
        u32::MAX
    } else {
        rank
    };
    (
        DateTime::from_timestamp(due, 0).unwrap_or(DateTime::<Utc>::MAX_UTC),
        rank,
    )
}

struct ScanKeys {
    lock: String,
    fresh: String,
    latest: String,
    progress: String,
    progress_id: String,
    schedule: String,
    refresher: String,
}

impl ScanKeys {
    fn new(config: &ScanConfig) -> Self {
        let mut base = format!(
            "{KEY_PREFIX}:scan:{}-{}-{}",
            config.rank_from, config.rank_to, config.max_details
        );
        if config.last_rank() > config.rank_to {
            base.push_str(&format!("-{}", config.last_rank()));
        }
        Self {
            lock: format!("{base}:lock"),
            fresh: format!("{base}:fresh"),
            latest: format!("{base}:latest"),
            progress: format!("{base}:progress"),
            progress_id: format!("{base}:progress-id"),
            schedule: format!("{base}:schedule"),
            refresher: format!("{base}:refresher"),
        }
    }
}

const TURN_SCRIPT: &str = r"
local ttl = redis.call('PTTL', KEYS[1])
if ttl > 0 then
  return {'fresh', tostring(ttl), redis.call('GET', KEYS[1])}
end
local progress = redis.call('GET', KEYS[3]) or ''
if redis.call('SET', KEYS[2], ARGV[1], 'NX', 'PX', ARGV[2]) then
  return {'lead', progress}
end
return {'wait', progress}
";

const PROGRESS_SCRIPT: &str = r"
if redis.call('GET', KEYS[1]) ~= ARGV[1] then
  return 0
end
redis.call('SET', KEYS[2], ARGV[2], 'PX', ARGV[3])
redis.call('SET', KEYS[3], ARGV[1], 'PX', ARGV[3])
return 1
";

const RENEW_SCRIPT: &str = r"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return redis.call('PEXPIRE', KEYS[1], ARGV[2])
end
return 0
";

const PUBLISH_SCRIPT: &str = r"
redis.call('SET', KEYS[1], ARGV[1], 'PX', ARGV[2])
redis.call('SET', KEYS[2], ARGV[3], 'PX', ARGV[4])
if redis.call('GET', KEYS[3]) == ARGV[3] then
  redis.call('DEL', KEYS[3])
end
redis.call('DEL', KEYS[4], KEYS[5])
return 1
";

const RELEASE_SCRIPT: &str = r"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return redis.call('DEL', KEYS[1])
end
return 0
";

const SCHEDULE_SCRIPT: &str = r"
local slots = tonumber(ARGV[2])
for i = 3, #ARGV, 2 do
  local due = tonumber(ARGV[1])
  local old = redis.call('ZSCORE', KEYS[1], ARGV[i])
  if old then
    due = math.floor(tonumber(old) / slots)
  end
  local score = due * slots + tonumber(ARGV[i + 1])
  redis.call('ZADD', KEYS[1], string.format('%.0f', score), ARGV[i])
end
return 1
";

const COVERAGE_SCRIPT: &str = r"
local ids = redis.call('ZRANGE', KEYS[1], 0, -1)
local loaded, expiring, shortest, total = 0, 0, -1, 0
for _, id in ipairs(ids) do
  local ttl = redis.call('PTTL', ARGV[1] .. id .. ARGV[2])
  if ttl ~= -2 then
    loaded = loaded + 1
  end
  if ttl >= 0 then
    expiring = expiring + 1
    total = total + ttl
    if shortest < 0 or ttl < shortest then
      shortest = ttl
    end
  end
end
local due = redis.call('ZCOUNT', KEYS[1], '-inf', ARGV[3])
return {#ids, loaded, due, expiring, shortest, total}
";

const MEDIANS_SCRIPT: &str = r"
for i = 2, #ARGV, 2 do
  redis.call('HSET', KEYS[1], ARGV[i], ARGV[i + 1])
end
redis.call('PEXPIRE', KEYS[1], ARGV[1])
return 1
";

const RESCHEDULE_SCRIPT: &str = r"
local old = redis.call('ZSCORE', KEYS[1], ARGV[1])
if not old then
  return 0
end
local slots = tonumber(ARGV[3])
local rank = tonumber(old) % slots
local score = tonumber(ARGV[2]) * slots + rank
redis.call('ZADD', KEYS[1], 'XX', string.format('%.0f', score), ARGV[1])
return 1
";

/// Figures from `INFO memory stats keyspace`, summed over every database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RedisStats {
    pub(crate) keys: u64,
    pub(crate) expiring: u64,
    pub(crate) average_ttl: Duration,
    pub(crate) used_memory: u64,
    pub(crate) max_memory: u64,
    pub(crate) evicted: u64,
    pub(crate) expired: u64,
}

impl RedisStats {
    /// Reads `INFO` output. Missing fields count as zero; `average_ttl` is weighted by each
    /// database's keys with a TTL.
    pub(crate) fn parse(info: &str) -> Self {
        let mut stats = Self::default();
        let mut ttl_weighted = 0u128;
        for line in info.lines() {
            let Some((name, value)) = line.trim().split_once(':') else {
                continue;
            };
            let number = || value.parse::<u64>().unwrap_or(0);
            match name {
                "used_memory" => stats.used_memory = number(),
                "maxmemory" => stats.max_memory = number(),
                "evicted_keys" => stats.evicted = number(),
                "expired_keys" => stats.expired = number(),
                db if db.starts_with("db") => {
                    let field = |key: &str| {
                        value
                            .split(',')
                            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
                            .and_then(|n| n.parse::<u64>().ok())
                            .unwrap_or(0)
                    };
                    let (keys, expires, avg_ttl) =
                        (field("keys"), field("expires"), field("avg_ttl"));
                    stats.keys += keys;
                    stats.expiring += expires;
                    ttl_weighted += u128::from(avg_ttl) * u128::from(expires);
                }
                _ => {}
            }
        }
        if stats.expiring > 0 {
            let average = ttl_weighted / u128::from(stats.expiring);
            stats.average_ttl = Duration::from_millis(u64::try_from(average).unwrap_or(u64::MAX));
        }
        stats
    }
}

fn redis_span(op: &'static str) -> Span {
    in_trace(|| {
        tracing::info_span!(
            "redis",
            otel.name = format!("redis {op}"),
            otel.kind = "client",
            db.system.name = "redis",
            db.operation.name = op,
            db.operation.batch.size = tracing::field::Empty,
        )
    })
}

fn observe(op: &'static str, started: Instant) {
    metrics::histogram!("redis_op_duration_seconds", "op" => op)
        .record(started.elapsed().as_secs_f64());
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

/// Shared-cache key for a club's details in a month (`None` = the current game month).
pub fn club_key(circle_id: u64, period: Option<Period>) -> String {
    format!("{KEY_PREFIX}:club:{circle_id}:{}", period_part(period))
}

fn searches_key() -> String {
    format!("{KEY_PREFIX}:searches")
}

fn medians_key(month: &str) -> String {
    format!("{KEY_PREFIX}:medians:{month}")
}

fn priors_key(month: &str) -> String {
    format!("{KEY_PREFIX}:priors:{month}")
}

fn period_part(period: Option<Period>) -> String {
    match period {
        None => "current".to_owned(),
        Some((year, month)) => format!("{year:04}-{month:02}"),
    }
}

fn kind(error: &redis::RedisError) -> String {
    format!("{:?}", error.kind())
}

// The first 128 bits of HMAC-SHA256: enough to avoid collisions, short enough for keys.
pub(crate) fn trainer_digest(secret: &KeySecret, viewer_id: u64) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.0.as_bytes()).expect("HMAC accepts any key length");
    mac.update(viewer_id.to_string().as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
