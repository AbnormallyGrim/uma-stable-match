//! Which club is linked to which Discord server, kept in Redis.
//!
//! - `discord:club:{club_id}`: a hash with the club's `server`, `category`, `auto_join`, `open`,
//!   `leader` (the Discord ID of the leader who last set it up, encrypted) and `problem`. Clubs linked before leader roles were dropped may still hold `leader_roles`,
//!   which nothing reads.
//! - `discord:guild:{guild_id}`: a set of the club IDs linked to that server.
//!
//! Neither key has a TTL, which keeps them out of `volatile-lru` eviction.

use std::collections::HashMap;
use std::time::Duration;

use redis::aio::ConnectionManager;
use redis::Script;
use tokio::time::timeout;

use crate::config::KeySecret;
use crate::sealed::Sealer;

const CLUB_PREFIX: &str = "discord:club:";
const GUILD_PREFIX: &str = "discord:guild:";
const LIMIT: Duration = Duration::from_secs(5);

/// Changes a club's settings, only while it is still linked to the server they were checked
/// against. ARGV is that server, then pairs of field and value; an empty value removes the field.
/// Returns 1 if it changed them.
const UPDATE: &str = r"
if redis.call('HGET', KEYS[1], 'server') ~= ARGV[1] then
  return 0
end
for i = 2, #ARGV, 2 do
  if ARGV[i + 1] == '' then
    redis.call('HDEL', KEYS[1], ARGV[i])
  else
    redis.call('HSET', KEYS[1], ARGV[i], ARGV[i + 1])
  end
end
return 1
";

/// Moves a club to a server. Returns the server it left and how many clubs are still linked there,
/// or nothing if it wasn't linked elsewhere. Changing servers drops the settings that name things
/// on the old one. The guild keys are built here from the prefix because the old server isn't
/// known until the club is read.
const LINK: &str = r"
local old = redis.call('HGET', KEYS[1], 'server')
local moved = old and old ~= ARGV[2]
if moved then
  redis.call('SREM', ARGV[3] .. old, ARGV[1])
  redis.call('HDEL', KEYS[1], 'category', 'leader_roles', 'problem')
end
redis.call('HSET', KEYS[1], 'server', ARGV[2])
redis.call('HSETNX', KEYS[1], 'open', '1')
redis.call('HSETNX', KEYS[1], 'auto_join', '1')
redis.call('SADD', ARGV[3] .. ARGV[2], ARGV[1])
if moved then
  return { old, tostring(redis.call('SCARD', ARGV[3] .. old)) }
end
return {}
";

/// Forgets a club. Returns the server it was linked to and how many clubs are still linked there,
/// or nothing if it wasn't linked.
const UNLINK: &str = r"
local old = redis.call('HGET', KEYS[1], 'server')
if not old then
  return {}
end
redis.call('DEL', KEYS[1])
redis.call('SREM', ARGV[2] .. old, ARGV[1])
return { old, tostring(redis.call('SCARD', ARGV[2] .. old)) }
";

/// A server a club has left, and how many clubs are still linked to it. At zero, nothing needs the
/// bot there any more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Departure {
    /// The Discord server's ID.
    pub guild_id: u64,
    /// Clubs still linked to it.
    pub remaining: u64,
}

/// What's kept about a linked club.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubRecord {
    /// The Discord server the club is linked to.
    pub guild_id: u64,
    /// The category for the club's channels, if set.
    pub category: Option<u64>,
    /// Whether applicants are added to the server automatically.
    pub auto_join: bool,
    /// Whether the club takes applications.
    pub open: bool,
    /// The problem with the club's setup, if there is one.
    pub problem: Option<String>,
    /// The Discord ID of the leader who last set the club up, who is added to its tickets.
    pub leader: Option<u64>,
}

/// Changes to a club's settings. A field left `None` stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsUpdate {
    /// The category for the club's channels; `Some(None)` clears it.
    pub category: Option<Option<u64>>,
    /// Whether applicants are added to the server automatically.
    pub auto_join: Option<bool>,
    /// Whether the club takes applications.
    pub open: Option<bool>,
    /// The Discord ID of the leader setting the club up.
    pub leader: Option<u64>,
}

impl SettingsUpdate {
    /// Field and value pairs for the update script.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        let flag = |on: bool| if on { "1" } else { "0" }.to_owned();
        let mut pairs = Vec::new();
        if let Some(category) = self.category {
            pairs.push((
                "category",
                category.map(|id| id.to_string()).unwrap_or_default(),
            ));
        }
        if let Some(on) = self.auto_join {
            pairs.push(("auto_join", flag(on)));
        }
        if let Some(on) = self.open {
            pairs.push(("open", flag(on)));
        }
        if let Some(leader) = self.leader {
            pairs.push(("leader", leader.to_string()));
        }
        pairs
    }
}

/// Why a store call failed. Holds the kind of failure, since Redis's messages can contain the URL.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Redis answered with an error.
    #[error("Redis error ({0})")]
    Redis(String),
    /// Redis didn't answer within 5 seconds.
    #[error("Redis timed out")]
    TimedOut,
}

impl From<redis::RedisError> for StoreError {
    fn from(error: redis::RedisError) -> Self {
        Self::Redis(format!("{:?}", error.kind()))
    }
}

/// The links between clubs and Discord servers. Cheap to clone.
#[derive(Clone)]
pub struct ClubStore {
    conn: ConnectionManager,
    sealer: Sealer,
    dev_guild: Option<u64>,
}

impl std::fmt::Debug for ClubStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClubStore").finish_non_exhaustive()
    }
}

impl ClubStore {
    /// A store using this connection, encrypting leaders' Discord IDs with a key from `secret`.
    pub fn new(conn: ConnectionManager, secret: &KeySecret) -> Self {
        Self {
            conn,
            sealer: Sealer::new(secret),
            dev_guild: None,
        }
    }

    /// The same store, answering as if every club took applications in this server. For trying
    /// applications on one's own machine.
    pub fn with_dev_guild(mut self, guild_id: u64) -> Self {
        self.dev_guild = Some(guild_id);
        self
    }

    /// Links a club to a server. A club is linked to one server at a time, so linking it to
    /// another moves it, dropping its category, leader roles and problem. Returns the server it
    /// left, if it moved.
    pub async fn link(&self, club_id: u64, guild_id: u64) -> Result<Option<Departure>, StoreError> {
        let script = Script::new(LINK);
        let mut invocation = script.key(club_key(club_id));
        invocation.arg(club_id).arg(guild_id).arg(GUILD_PREFIX);
        let reply: Vec<String> = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(departure(&reply))
    }

    /// Unlinks a club, deleting everything kept about it. Returns the server it was linked to, if
    /// it was linked.
    pub async fn unlink(&self, club_id: u64) -> Result<Option<Departure>, StoreError> {
        let script = Script::new(UNLINK);
        let mut invocation = script.key(club_key(club_id));
        invocation.arg(club_id).arg(GUILD_PREFIX);
        let reply: Vec<String> = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(departure(&reply))
    }

    /// Applies `update` to a club, if it is still linked to `guild_id`. Returns whether it did, so
    /// a club that moved or was unlinked meanwhile isn't changed.
    pub async fn update(
        &self,
        club_id: u64,
        guild_id: u64,
        update: &SettingsUpdate,
    ) -> Result<bool, StoreError> {
        let script = Script::new(UPDATE);
        let mut invocation = script.key(club_key(club_id));
        invocation.arg(guild_id);
        for (field, value) in update.pairs() {
            let value = if field == "leader" {
                self.sealer.seal(&value, &leader_context(club_id))
            } else {
                value
            };
            invocation.arg(field).arg(value);
        }
        let changed: i64 = self
            .run(invocation.invoke_async(&mut self.conn.clone()))
            .await?;
        Ok(changed == 1)
    }

    /// For each club, whether it is linked, takes applications and has no setup problem.
    pub async fn takes_applications(&self, club_ids: &[u64]) -> Result<Vec<bool>, StoreError> {
        if self.dev_guild.is_some() {
            return Ok(vec![true; club_ids.len()]);
        }
        if club_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut pipeline = redis::pipe();
        for club_id in club_ids {
            pipeline
                .cmd("HMGET")
                .arg(club_key(*club_id))
                .arg("server")
                .arg("open")
                .arg("problem");
        }
        let rows: Vec<(Option<String>, Option<String>, Option<String>)> = self
            .run(pipeline.query_async(&mut self.conn.clone()))
            .await?;
        Ok(rows
            .into_iter()
            .map(|(server, open, problem)| {
                server.is_some()
                    && open.as_deref() == Some("1")
                    && problem.is_none_or(|p| p.is_empty())
            })
            .collect())
    }

    /// What's kept about a club, or `None` if it isn't linked.
    pub async fn club(&self, club_id: u64) -> Result<Option<ClubRecord>, StoreError> {
        if let Some(guild_id) = self.dev_guild {
            return Ok(Some(ClubRecord {
                guild_id,
                category: None,
                auto_join: true,
                open: true,
                problem: None,
                leader: None,
            }));
        }
        let fields: HashMap<String, String> = self
            .run(
                redis::cmd("HGETALL")
                    .arg(club_key(club_id))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(parse_record(&self.sealer, club_id, &fields))
    }

    /// The clubs linked to a server, lowest ID first.
    pub async fn clubs_on(&self, guild_id: u64) -> Result<Vec<u64>, StoreError> {
        let members: Vec<String> = self
            .run(
                redis::cmd("SMEMBERS")
                    .arg(format!("{GUILD_PREFIX}{guild_id}"))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        let mut clubs: Vec<u64> = members.iter().filter_map(|id| id.parse().ok()).collect();
        clubs.sort_unstable();
        Ok(clubs)
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

fn club_key(club_id: u64) -> String {
    format!("{CLUB_PREFIX}{club_id}")
}

fn departure(reply: &[String]) -> Option<Departure> {
    match reply {
        [guild, remaining] => Some(Departure {
            guild_id: guild.parse().ok()?,
            remaining: remaining.parse().ok()?,
        }),
        _ => None,
    }
}

/// Ties an encrypted leader to its club, so it can't be copied to another.
fn leader_context(club_id: u64) -> String {
    format!("club:{club_id}:leader")
}

/// The leader's Discord ID: encrypted, or a plain number from before leaders were encrypted.
fn leader(sealer: &Sealer, club_id: u64, value: &str) -> Option<u64> {
    value
        .parse()
        .ok()
        .or_else(|| sealer.open(value, &leader_context(club_id))?.parse().ok())
}

fn parse_record(
    sealer: &Sealer,
    club_id: u64,
    fields: &HashMap<String, String>,
) -> Option<ClubRecord> {
    let guild_id = fields.get("server")?.parse().ok()?;
    Some(ClubRecord {
        guild_id,
        category: fields.get("category").and_then(|id| id.parse().ok()),
        auto_join: fields.get("auto_join").is_some_and(|v| v == "1"),
        open: fields.get("open").is_some_and(|v| v == "1"),
        problem: fields.get("problem").filter(|v| !v.is_empty()).cloned(),
        leader: fields
            .get("leader")
            .and_then(|value| leader(sealer, club_id, value)),
    })
}

#[cfg(test)]
mod tests;
