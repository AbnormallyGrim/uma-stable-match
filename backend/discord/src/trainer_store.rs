//! Which trainer IDs a Discord user has proved they own, kept in Redis.
//!
//! - `discord:trainer:{trainer_id}`: the hash of the Discord ID that owns the trainer ID.
//! - `discord:applicant:{hash}`: a set of the trainer IDs that Discord ID owns.
//! - `discord:verify:{hash}:{trainer_id}`: the key a user was told to put in their profile, until
//!   it is used or expires.
//! - `discord:hits:{action}:{hash}`: how often a user tried an action lately.
//!
//! `{hash}` is a keyed hash of the Discord ID, so Redis holds no Discord IDs. The first two have no
//! TTL, so eviction never removes them; they go when the user unlinks. The others expire.

use std::time::Duration;

use hmac::{Hmac, KeyInit, Mac};
use redis::aio::ConnectionManager;
use redis::Script;
use sha2::Sha256;
use tokio::time::timeout;

use crate::config::KeySecret;
use crate::store::StoreError;

const TRAINER_PREFIX: &str = "discord:trainer:";
const APPLICANT_PREFIX: &str = "discord:applicant:";
const PENDING_PREFIX: &str = "discord:verify:";
const HITS_PREFIX: &str = "discord:hits:";
const LIMIT: Duration = Duration::from_secs(5);

/// How long a verification key can be used for.
pub const KEY_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// Most trainer IDs one Discord user can have linked.
pub const MAX_LINKED_TRAINERS: usize = 5;

const WINDOW: u64 = 60 * 60;

/// Starts a verification: stores the key unless one is pending. Returns the pending key and the
/// seconds it has left.
const START: &str = r"
if redis.call('SET', KEYS[1], ARGV[1], 'NX', 'EX', ARGV[2]) then
  return { ARGV[1], ARGV[2] }
end
return { redis.call('GET', KEYS[1]), tostring(redis.call('TTL', KEYS[1])) }
";

/// Links a trainer ID to a user, taking it from any other owner. KEYS are the trainer's key, the
/// user's set and the pending key; ARGV the user's hash, the trainer ID, the most IDs per user
/// and the set prefix. Returns 0, changing nothing, if the user has no room for another ID.
const LINK: &str = r"
if redis.call('SISMEMBER', KEYS[2], ARGV[2]) == 0
    and redis.call('SCARD', KEYS[2]) >= tonumber(ARGV[3]) then
  return 0
end
local owner = redis.call('GET', KEYS[1])
if owner and owner ~= ARGV[1] then
  redis.call('SREM', ARGV[4] .. owner, ARGV[2])
end
redis.call('SET', KEYS[1], ARGV[1])
redis.call('SADD', KEYS[2], ARGV[2])
redis.call('DEL', KEYS[3])
return 1
";

/// Removes a trainer ID from a user. Returns 1 if the user owned it.
const UNLINK: &str = r"
redis.call('SREM', KEYS[2], ARGV[2])
if redis.call('GET', KEYS[1]) == ARGV[1] then
  redis.call('DEL', KEYS[1])
  return 1
end
return 0
";

/// Counts an attempt. Returns the attempts this hour and the seconds until the count resets.
const HIT: &str = r"
local n = redis.call('INCR', KEYS[1])
if n == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return { n, redis.call('TTL', KEYS[1]) }
";

/// What a user may try only so often, as each try costs a call to uma.moe or a key in Redis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Asking for a verification key: 20 an hour.
    Start,
    /// Checking a profile for the key: 10 an hour.
    Check,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Check => "check",
        }
    }

    fn per_hour(self) -> i64 {
        match self {
            Self::Start => 20,
            Self::Check => 10,
        }
    }
}

/// A verification key waiting to be found in a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The key to put in the profile comment.
    pub key: String,
    /// Seconds before it expires.
    pub expires_in: u64,
}

/// What linking a trainer ID did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// The trainer ID now belongs to the user.
    Linked,
    /// The user already has [`MAX_LINKED_TRAINERS`] IDs linked; nothing changed.
    TooMany,
}

/// The links between Discord users and the trainer IDs they own. Cheap to clone.
#[derive(Clone)]
pub struct TrainerStore {
    conn: ConnectionManager,
    secret: KeySecret,
}

impl std::fmt::Debug for TrainerStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainerStore").finish_non_exhaustive()
    }
}

impl TrainerStore {
    /// A store using this connection, hashing Discord IDs with `secret`.
    pub fn new(conn: ConnectionManager, secret: KeySecret) -> Self {
        Self { conn, secret }
    }

    /// The key `user` should put in the profile of `trainer`: the one already pending, or `key`
    /// if none is.
    pub async fn start(&self, user: u64, trainer: u64, key: &str) -> Result<Pending, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        let script = Script::new(START);
        let mut invocation = script.key(pending_key(&hash, trainer));
        invocation.arg(key).arg(KEY_LIFETIME.as_secs());
        let (key, expires_in): (String, i64) = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(Pending {
            key,
            expires_in: expires_in.max(0) as u64,
        })
    }

    /// The key `user` has pending for `trainer`, if any.
    pub async fn pending(&self, user: u64, trainer: u64) -> Result<Option<String>, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        self.run(
            redis::cmd("GET")
                .arg(pending_key(&hash, trainer))
                .query_async(&mut self.conn.clone()),
        )
        .await
    }

    /// Links `trainer` to `user`, taking it from whoever owned it, and forgets the pending key.
    pub async fn link(&self, user: u64, trainer: u64) -> Result<LinkOutcome, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        let script = Script::new(LINK);
        let mut invocation = script.key(format!("{TRAINER_PREFIX}{trainer}"));
        invocation
            .key(format!("{APPLICANT_PREFIX}{hash}"))
            .key(pending_key(&hash, trainer));
        invocation
            .arg(&hash)
            .arg(trainer)
            .arg(MAX_LINKED_TRAINERS)
            .arg(APPLICANT_PREFIX);
        let linked: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(if linked == 1 {
            LinkOutcome::Linked
        } else {
            LinkOutcome::TooMany
        })
    }

    /// Unlinks `trainer` from `user`. Returns whether `user` owned it.
    pub async fn unlink(&self, user: u64, trainer: u64) -> Result<bool, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        let script = Script::new(UNLINK);
        let mut invocation = script.key(format!("{TRAINER_PREFIX}{trainer}"));
        invocation.key(format!("{APPLICANT_PREFIX}{hash}"));
        invocation.arg(&hash).arg(trainer);
        let owned: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(owned == 1)
    }

    /// The trainer IDs `user` owns, lowest first.
    pub async fn linked(&self, user: u64) -> Result<Vec<u64>, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        let members: Vec<String> = self
            .run(
                redis::cmd("SMEMBERS")
                    .arg(format!("{APPLICANT_PREFIX}{hash}"))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        let mut trainers: Vec<u64> = members.iter().filter_map(|id| id.parse().ok()).collect();
        trainers.sort_unstable();
        Ok(trainers)
    }

    /// Counts an attempt by `user`. Returns `None` if it's allowed, or how many seconds to wait
    /// once the user has tried too often this hour.
    pub async fn hit(&self, user: u64, action: Action) -> Result<Option<u64>, StoreError> {
        let hash = applicant_hash(&self.secret, user);
        let script = Script::new(HIT);
        let mut invocation = script.key(format!("{HITS_PREFIX}{}:{hash}", action.label()));
        invocation.arg(WINDOW);
        let (count, ttl): (i64, i64) = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok((count > action.per_hour()).then(|| ttl.max(1) as u64))
    }

    async fn run<T>(
        &self,
        call: impl std::future::Future<Output = redis::RedisResult<T>>,
    ) -> Result<T, StoreError> {
        timeout(LIMIT, call)
            .await
            .map_err(|_| StoreError::TimedOut)?
            .map_err(StoreError::from)
    }
}

fn pending_key(hash: &str, trainer: u64) -> String {
    format!("{PENDING_PREFIX}{hash}:{trainer}")
}

/// A keyed hash of a Discord ID, so Redis never holds the ID itself.
pub(crate) fn applicant_hash(secret: &KeySecret, user: u64) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("discord.applicant.v1.{user}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
