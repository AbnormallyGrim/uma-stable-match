//! Which trainer IDs a Discord user has proved they own, kept in Redis.
//!
//! - `discord:trainer:{trainer_id}`: the hash of the Discord ID that owns the trainer ID.
//! - `discord:applicant:{hash}`: a set of the trainer IDs that Discord ID owns.
//! - `discord:names:{hash}`: the names of those trainers, by trainer ID.
//! - `discord:verify:{hash}:{trainer_id}`: the key a user was told to put in their profile, until
//!   it is used or expires.
//! - `discord:pending:{hash}`: the trainer IDs a user has keys pending for.
//! - `discord:key:{key}`: marks a pending key as taken, so no two people are given the same one.
//! - `discord:check:{hash}:{trainer_id}`: a running or finished check for the key.
//! - `discord:checks`: the running checks, scored by when to look again.
//! - `discord:hits:{action}:{hash}`: how often a user tried an action lately.
//!
//! `{hash}` is a keyed hash of the Discord ID. The first three keys have no TTL and are deleted
//! when the user unlinks or deletes their account. The others expire or empty themselves.

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
const NAMES_PREFIX: &str = "discord:names:";
const PENDING_PREFIX: &str = "discord:verify:";
const PENDING_SET_PREFIX: &str = "discord:pending:";
const TAKEN_PREFIX: &str = "discord:key:";
const CHECK_PREFIX: &str = "discord:check:";
const CHECKS: &str = "discord:checks";
const HITS_PREFIX: &str = "discord:hits:";
const LIMIT: Duration = Duration::from_secs(5);

/// How long a verification key can be used for.
pub const KEY_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// Most trainer IDs one Discord user can have linked.
pub const MAX_LINKED_TRAINERS: usize = 5;

/// How long a finished check's outcome is kept, for a page opened later to show.
const OUTCOME_LIFETIME: u64 = 15 * 60;

/// Most running checks one pass claims.
const CLAIM_BATCH: usize = 20;

const WINDOW: u64 = 60 * 60;

/// Starts a verification: stores the key unless one is pending. KEYS are the pending key, the
/// new key's taken marker and the user's pending set; ARGV the new key, its lifetime and the
/// trainer ID. Returns the pending key and the seconds it has left, or nothing if someone else
/// holds the new key.
const START: &str = r"
local pending = redis.call('GET', KEYS[1])
if pending then
  return { pending, tostring(redis.call('TTL', KEYS[1])) }
end
if not redis.call('SET', KEYS[2], KEYS[1], 'NX', 'EX', ARGV[2]) then
  return false
end
redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2])
redis.call('SADD', KEYS[3], ARGV[3])
redis.call('EXPIRE', KEYS[3], ARGV[2])
return { ARGV[1], ARGV[2] }
";

/// Links a trainer ID to a user, taking it from any other owner. KEYS are the trainer's key, the
/// user's set, the pending key, the user's names and the user's pending set; ARGV the user's
/// hash, the trainer ID, the most IDs per user, the set prefix, the taken prefix, the names
/// prefix and the trainer's name. Returns 0, changing nothing, if the user has no room.
const LINK: &str = r"
if redis.call('SISMEMBER', KEYS[2], ARGV[2]) == 0
    and redis.call('SCARD', KEYS[2]) >= tonumber(ARGV[3]) then
  return 0
end
local owner = redis.call('GET', KEYS[1])
if owner and owner ~= ARGV[1] then
  redis.call('SREM', ARGV[4] .. owner, ARGV[2])
  redis.call('HDEL', ARGV[6] .. owner, ARGV[2])
end
redis.call('SET', KEYS[1], ARGV[1])
redis.call('SADD', KEYS[2], ARGV[2])
redis.call('HSET', KEYS[4], ARGV[2], ARGV[7])
local key = redis.call('GET', KEYS[3])
if key then
  redis.call('DEL', KEYS[3], ARGV[5] .. key)
end
redis.call('SREM', KEYS[5], ARGV[2])
return 1
";

/// Removes a trainer ID from a user. KEYS are the trainer's key, the user's set and names.
/// Returns 1 if the user owned it.
const UNLINK: &str = r"
redis.call('SREM', KEYS[2], ARGV[2])
redis.call('HDEL', KEYS[3], ARGV[2])
if redis.call('GET', KEYS[1]) == ARGV[1] then
  redis.call('DEL', KEYS[1])
  return 1
end
return 0
";

/// Forgets a pending key and any check for it. KEYS are the pending key, the pending set, the
/// check and the queue; ARGV the trainer ID, the queue member and the taken prefix.
const DISCARD: &str = r"
local key = redis.call('GET', KEYS[1])
if key then
  redis.call('DEL', KEYS[1], ARGV[3] .. key)
end
redis.call('SREM', KEYS[2], ARGV[1])
redis.call('DEL', KEYS[3])
redis.call('ZREM', KEYS[4], ARGV[2])
return 1
";

/// Removes every trainer ID a user owns, their names, pending set and attempt counters. KEYS are
/// the user's set, two counters, names and pending set; ARGV the user's hash and the trainer
/// prefix.
const FORGET: &str = r"
for _, trainer in ipairs(redis.call('SMEMBERS', KEYS[1])) do
  local key = ARGV[2] .. trainer
  if redis.call('GET', key) == ARGV[1] then
    redis.call('DEL', key)
  end
end
redis.call('DEL', KEYS[1], KEYS[2], KEYS[3], KEYS[4], KEYS[5])
return 1
";

/// Starts a check unless one is running. KEYS are the check and the queue; ARGV when it ends,
/// when to look again, the queue member and how long to keep the check. Returns 0 if one is
/// already running.
const BEGIN_CHECK: &str = r"
if redis.call('HGET', KEYS[1], 'state') == 'checking' then
  return 0
end
redis.call('DEL', KEYS[1])
redis.call('HSET', KEYS[1], 'state', 'checking', 'until', ARGV[1])
redis.call('EXPIRE', KEYS[1], ARGV[4])
redis.call('ZADD', KEYS[2], ARGV[2], ARGV[3])
return 1
";

/// Claims the checks due by ARGV[1], moving each to ARGV[2] so no other server takes it
/// meanwhile. KEYS is the queue; ARGV[3] the most to claim.
const CLAIM: &str = r"
local due = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[3])
for _, member in ipairs(due) do
  redis.call('ZADD', KEYS[1], ARGV[2], member)
end
return due
";

/// Counts an attempt. Returns the attempts this hour and the seconds until the count resets.
const HIT: &str = r"
local n = redis.call('INCR', KEYS[1])
if n == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return { n, redis.call('TTL', KEYS[1]) }
";

/// A rate-limited user action. Each try costs a call to uma.moe or a key in Redis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Asking for a verification key: 20 an hour.
    Start,
    /// Starting a check of a profile for the key: 10 an hour.
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

/// A pending key, the trainer it's for, and its check if one ran lately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLink {
    /// The trainer ID.
    pub trainer: u64,
    /// The key and how long it lasts.
    pub pending: Pending,
    /// The running or last check.
    pub check: Option<CheckState>,
}

/// Where a check for a key stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckState {
    /// Still looking, until this Unix time.
    Checking {
        /// When the last try is due, in Unix seconds.
        until: u64,
    },
    /// The key never showed up in the comment.
    Missing,
    /// uma.moe doesn't know the trainer.
    NotFound,
    /// The key showed up, but the user has no room for another trainer.
    TooMany,
}

impl CheckState {
    fn label(&self) -> &'static str {
        match self {
            Self::Checking { .. } => "checking",
            Self::Missing => "missing",
            Self::NotFound => "not_found",
            Self::TooMany => "too_many",
        }
    }

    fn parse(state: &str, until: Option<u64>) -> Option<Self> {
        match state {
            "checking" => until.map(|until| Self::Checking { until }),
            "missing" => Some(Self::Missing),
            "not_found" => Some(Self::NotFound),
            "too_many" => Some(Self::TooMany),
            _ => None,
        }
    }
}

/// A linked trainer ID and the name it had when it linked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedTrainer {
    /// The trainer ID.
    pub trainer: u64,
    /// The trainer's name, if it was kept.
    pub name: Option<String>,
}

/// What linking a trainer ID did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// The trainer ID now belongs to the user.
    Linked,
    /// The user already has [`MAX_LINKED_TRAINERS`] IDs linked; nothing changed.
    TooMany,
}

/// A running check the queue handed out: whose it is and for which trainer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueCheck {
    pub(crate) hash: String,
    pub(crate) trainer: u64,
}

impl DueCheck {
    fn member(&self) -> String {
        format!("{}:{}", self.hash, self.trainer)
    }

    fn parse(member: &str) -> Option<Self> {
        let (hash, trainer) = member.rsplit_once(':')?;
        Some(Self {
            hash: hash.to_owned(),
            trainer: trainer.parse().ok()?,
        })
    }
}

/// The links between Discord users and the trainer IDs they own. Cheap to clone.
#[derive(Clone)]
pub struct TrainerStore {
    conn: ConnectionManager,
    secret: KeySecret,
    checks: std::sync::Arc<str>,
}

impl std::fmt::Debug for TrainerStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainerStore").finish_non_exhaustive()
    }
}

impl TrainerStore {
    /// A store using this connection, hashing Discord IDs with `secret`.
    pub fn new(conn: ConnectionManager, secret: KeySecret) -> Self {
        Self {
            conn,
            secret,
            checks: CHECKS.into(),
        }
    }

    /// The same store with its queue of running checks under another Redis key, so that apps
    /// sharing one Redis, such as tests, don't take each other's checks.
    pub fn with_check_queue(mut self, key: &str) -> Self {
        self.checks = key.into();
        self
    }

    pub(crate) fn hash(&self, user: u64) -> String {
        applicant_hash(&self.secret, user)
    }

    /// The key `user` should put in the profile of `trainer`: the one already pending, or `key`
    /// if none is. `None` if `key` is pending for someone else; try another.
    pub async fn start(
        &self,
        user: u64,
        trainer: u64,
        key: &str,
    ) -> Result<Option<Pending>, StoreError> {
        let hash = self.hash(user);
        let script = Script::new(START);
        let mut invocation = script.key(pending_key(&hash, trainer));
        invocation
            .key(taken_key(key))
            .key(format!("{PENDING_SET_PREFIX}{hash}"));
        invocation.arg(key).arg(KEY_LIFETIME.as_secs()).arg(trainer);
        let started: Option<(String, i64)> = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(started.map(|(key, expires_in)| Pending {
            key,
            expires_in: expires_in.max(0) as u64,
        }))
    }

    /// The key `user` has pending for `trainer`, if any.
    pub async fn pending(&self, user: u64, trainer: u64) -> Result<Option<String>, StoreError> {
        self.pending_for(&self.hash(user), trainer).await
    }

    pub(crate) async fn pending_for(
        &self,
        hash: &str,
        trainer: u64,
    ) -> Result<Option<String>, StoreError> {
        self.run(
            redis::cmd("GET")
                .arg(pending_key(hash, trainer))
                .query_async(&mut self.conn.clone()),
        )
        .await
    }

    /// Every key `user` has pending, longest-lasting (newest) first, with any recent check.
    pub async fn pending_links(&self, user: u64) -> Result<Vec<PendingLink>, StoreError> {
        let hash = self.hash(user);
        let members: Vec<String> = self
            .run(
                redis::cmd("SMEMBERS")
                    .arg(format!("{PENDING_SET_PREFIX}{hash}"))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        let mut found = Vec::new();
        for trainer in members.iter().filter_map(|id| id.parse::<u64>().ok()) {
            let key = pending_key(&hash, trainer);
            let (pending, ttl, state, until): (Option<String>, i64, Option<String>, Option<u64>) =
                self.run(
                    redis::pipe()
                        .cmd("GET")
                        .arg(&key)
                        .cmd("TTL")
                        .arg(&key)
                        .cmd("HGET")
                        .arg(check_key(&hash, trainer))
                        .arg("state")
                        .cmd("HGET")
                        .arg(check_key(&hash, trainer))
                        .arg("until")
                        .query_async(&mut self.conn.clone()),
                )
                .await?;
            if let Some(key) = pending {
                found.push(PendingLink {
                    trainer,
                    pending: Pending {
                        key,
                        expires_in: ttl.max(0) as u64,
                    },
                    check: state.and_then(|state| CheckState::parse(&state, until)),
                });
            }
        }
        found.sort_by_key(|link| std::cmp::Reverse(link.pending.expires_in));
        Ok(found)
    }

    /// Forgets the key `user` has pending for `trainer`, stopping any check for it.
    pub async fn discard(&self, user: u64, trainer: u64) -> Result<(), StoreError> {
        let hash = self.hash(user);
        let due = DueCheck {
            hash: hash.clone(),
            trainer,
        };
        let script = Script::new(DISCARD);
        let mut invocation = script.key(pending_key(&hash, trainer));
        invocation
            .key(format!("{PENDING_SET_PREFIX}{hash}"))
            .key(check_key(&hash, trainer))
            .key(&*self.checks);
        invocation.arg(trainer).arg(due.member()).arg(TAKEN_PREFIX);
        let _: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(())
    }

    /// Links `trainer`, named `name`, to `user`, taking it from whoever owned it, and forgets the
    /// pending key.
    pub async fn link(
        &self,
        user: u64,
        trainer: u64,
        name: &str,
    ) -> Result<LinkOutcome, StoreError> {
        self.link_for(&self.hash(user), trainer, name).await
    }

    pub(crate) async fn link_for(
        &self,
        hash: &str,
        trainer: u64,
        name: &str,
    ) -> Result<LinkOutcome, StoreError> {
        let script = Script::new(LINK);
        let mut invocation = script.key(format!("{TRAINER_PREFIX}{trainer}"));
        invocation
            .key(format!("{APPLICANT_PREFIX}{hash}"))
            .key(pending_key(hash, trainer))
            .key(format!("{NAMES_PREFIX}{hash}"))
            .key(format!("{PENDING_SET_PREFIX}{hash}"));
        invocation
            .arg(hash)
            .arg(trainer)
            .arg(MAX_LINKED_TRAINERS)
            .arg(APPLICANT_PREFIX)
            .arg(TAKEN_PREFIX)
            .arg(NAMES_PREFIX)
            .arg(name);
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
        let hash = self.hash(user);
        let script = Script::new(UNLINK);
        let mut invocation = script.key(format!("{TRAINER_PREFIX}{trainer}"));
        invocation
            .key(format!("{APPLICANT_PREFIX}{hash}"))
            .key(format!("{NAMES_PREFIX}{hash}"));
        invocation.arg(&hash).arg(trainer);
        let owned: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(owned == 1)
    }

    /// The trainer IDs `user` owns, lowest first.
    pub async fn linked(&self, user: u64) -> Result<Vec<u64>, StoreError> {
        Ok(self
            .linked_trainers(user)
            .await?
            .into_iter()
            .map(|linked| linked.trainer)
            .collect())
    }

    /// The trainer IDs `user` owns with their names, lowest ID first.
    pub async fn linked_trainers(&self, user: u64) -> Result<Vec<LinkedTrainer>, StoreError> {
        let hash = self.hash(user);
        let members: Vec<String> = self
            .run(
                redis::cmd("SMEMBERS")
                    .arg(format!("{APPLICANT_PREFIX}{hash}"))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        let mut trainers: Vec<u64> = members.iter().filter_map(|id| id.parse().ok()).collect();
        trainers.sort_unstable();
        if trainers.is_empty() {
            return Ok(Vec::new());
        }
        let names: Vec<Option<String>> = self
            .run(
                redis::cmd("HMGET")
                    .arg(format!("{NAMES_PREFIX}{hash}"))
                    .arg(&trainers)
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(trainers
            .into_iter()
            .zip(names)
            .map(|(trainer, name)| LinkedTrainer { trainer, name })
            .collect())
    }

    /// Deletes everything kept about `user`: their trainer IDs and names, pending keys (and their
    /// taken markers and checks) and attempt counts.
    pub async fn forget(&self, user: u64) -> Result<(), StoreError> {
        let hash = self.hash(user);
        let script = Script::new(FORGET);
        let mut invocation = script.key(format!("{APPLICANT_PREFIX}{hash}"));
        for action in [Action::Start, Action::Check] {
            invocation.key(hits_key(action, &hash));
        }
        invocation
            .key(format!("{NAMES_PREFIX}{hash}"))
            .key(format!("{PENDING_SET_PREFIX}{hash}"));
        invocation.arg(&hash).arg(TRAINER_PREFIX);
        let _: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        // Pending keys are named by trainer ID, so they're found by scanning.
        let pattern = format!("{PENDING_PREFIX}{hash}:*");
        let mut cursor = 0u64;
        loop {
            let (next, keys): (u64, Vec<String>) = self
                .run(
                    redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(&pattern)
                        .arg("COUNT")
                        .arg(100)
                        .query_async(&mut self.conn.clone()),
                )
                .await?;
            for key in &keys {
                if let Some(trainer) = key.rsplit(':').next().and_then(|id| id.parse().ok()) {
                    self.discard(user, trainer).await?;
                }
            }
            if next == 0 {
                return Ok(());
            }
            cursor = next;
        }
    }

    /// The running or last check of `user`'s key for `trainer`.
    pub async fn check_state(
        &self,
        user: u64,
        trainer: u64,
    ) -> Result<Option<CheckState>, StoreError> {
        let (state, until): (Option<String>, Option<u64>) = self
            .run(
                redis::cmd("HMGET")
                    .arg(check_key(&self.hash(user), trainer))
                    .arg("state")
                    .arg("until")
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(state.and_then(|state| CheckState::parse(&state, until)))
    }

    /// Starts a check of `user`'s key for `trainer` that runs until `until`, looking next at
    /// `next` (Unix seconds). Returns `false` if one is already running.
    pub async fn begin_check(
        &self,
        user: u64,
        trainer: u64,
        until: u64,
        next: u64,
    ) -> Result<bool, StoreError> {
        let hash = self.hash(user);
        let due = DueCheck {
            hash: hash.clone(),
            trainer,
        };
        let script = Script::new(BEGIN_CHECK);
        let mut invocation = script.key(check_key(&hash, trainer));
        invocation.key(&*self.checks);
        invocation
            .arg(until)
            .arg(next)
            .arg(due.member())
            .arg(until.saturating_sub(next) + 60 + OUTCOME_LIFETIME);
        let begun: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(begun == 1)
    }

    /// The checks due by `now`, each held back for `lease` seconds so no one else takes it.
    pub(crate) async fn claim_due(
        &self,
        now: u64,
        lease: u64,
    ) -> Result<Vec<DueCheck>, StoreError> {
        let script = Script::new(CLAIM);
        let mut invocation = script.key(&*self.checks);
        invocation.arg(now).arg(now + lease).arg(CLAIM_BATCH);
        let members: Vec<String> = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(members.iter().filter_map(|m| DueCheck::parse(m)).collect())
    }

    /// When the check for `due` ends, if it's still running.
    pub(crate) async fn running_until(&self, due: &DueCheck) -> Result<Option<u64>, StoreError> {
        let (state, until): (Option<String>, Option<u64>) = self
            .run(
                redis::cmd("HMGET")
                    .arg(check_key(&due.hash, due.trainer))
                    .arg("state")
                    .arg("until")
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(match (state.as_deref(), until) {
            (Some("checking"), Some(until)) => Some(until),
            _ => None,
        })
    }

    /// Looks at `due` again at `next`.
    pub(crate) async fn retry_at(&self, due: &DueCheck, next: u64) -> Result<(), StoreError> {
        self.run(
            redis::cmd("ZADD")
                .arg(&*self.checks)
                .arg(next)
                .arg(due.member())
                .query_async::<i64>(&mut self.conn.clone()),
        )
        .await?;
        Ok(())
    }

    /// Ends the check for `due`, keeping `outcome` (or nothing, once it linked) for a while.
    pub(crate) async fn finish(
        &self,
        due: &DueCheck,
        outcome: Option<CheckState>,
    ) -> Result<(), StoreError> {
        let key = check_key(&due.hash, due.trainer);
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("ZREM")
            .arg(&*self.checks)
            .arg(due.member());
        match outcome {
            Some(state) => {
                pipe.cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(state.label())
                    .cmd("EXPIRE")
                    .arg(&key)
                    .arg(OUTCOME_LIFETIME);
            }
            None => {
                pipe.cmd("DEL").arg(&key);
            }
        }
        self.run(pipe.query_async::<()>(&mut self.conn.clone()))
            .await
    }

    /// Counts an attempt by `user`. Returns `None` if it's allowed, or how many seconds to wait
    /// once the user has tried too often this hour.
    pub async fn hit(&self, user: u64, action: Action) -> Result<Option<u64>, StoreError> {
        let hash = self.hash(user);
        let script = Script::new(HIT);
        let mut invocation = script.key(hits_key(action, &hash));
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

fn hits_key(action: Action, hash: &str) -> String {
    format!("{HITS_PREFIX}{}:{hash}", action.label())
}

/// The marker for `key`, as it was made: `LINK` builds the same name from the stored key.
fn taken_key(key: &str) -> String {
    format!("{TAKEN_PREFIX}{key}")
}

fn pending_key(hash: &str, trainer: u64) -> String {
    format!("{PENDING_PREFIX}{hash}:{trainer}")
}

fn check_key(hash: &str, trainer: u64) -> String {
    format!("{CHECK_PREFIX}{hash}:{trainer}")
}

/// A keyed hash of a Discord ID, which Redis stores in place of the ID.
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
