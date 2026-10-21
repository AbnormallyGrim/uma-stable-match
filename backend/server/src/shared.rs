//! Optional cache shared between server instances.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hmac::{Hmac, KeyInit, Mac};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, FromRedisValue, Script, ScriptInvocation};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Sha256;
use tokio::time::timeout;

use crate::pool::Period;
use crate::scanner::ScanConfig;

const KEY_PREFIX: &str = "usm:v1";
const MIN_SECRET_LEN: usize = 32;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OP_TIMEOUT: Duration = Duration::from_millis(500);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
const SCAN_LEASE: Duration = Duration::from_secs(60);

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
        let reply = timeout(limit, invocation.invoke_async(&mut conn)).await;
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
        let reply = timeout(limit, conn.get::<_, Option<Vec<u8>>>(key)).await;
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
        .await;
        observe("set", started);
        match reply {
            Ok(Ok(())) => redis.up(),
            Ok(Err(error)) => redis.down(&kind(&error)),
            Err(_) => redis.down("timed out"),
        }
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

    /// The last scan published for this scan window.
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
    /// This instance holds the scan lock: scan, then publish.
    Lead {
        lease: ScanLease,
        progress: Option<String>,
    },
    /// Another instance is scanning: check again shortly.
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

    /// Stores `value` for two intervals, marks it current for one, and releases the lock.
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

struct ScanKeys {
    lock: String,
    fresh: String,
    latest: String,
    progress: String,
    progress_id: String,
}

impl ScanKeys {
    fn new(config: &ScanConfig) -> Self {
        let base = format!(
            "{KEY_PREFIX}:scan:{}-{}-{}",
            config.rank_from, config.rank_to, config.max_details
        );
        Self {
            lock: format!("{base}:lock"),
            fresh: format!("{base}:fresh"),
            latest: format!("{base}:latest"),
            progress: format!("{base}:progress"),
            progress_id: format!("{base}:progress-id"),
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
