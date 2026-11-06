//! Applications to clubs, kept in Redis until a calendar month after they're sent.
//!
//! - `discord:app:{id}`: a hash with the application's fields.
//! - `discord:club-apps:{club_id}`: the club's application IDs, scored by when they were sent.
//! - `discord:my-apps:{hash}`: the same for one applicant, with `{hash}` a keyed hash of their
//!   Discord ID.
//! - `discord:applied:{hash}:{club_id}`: the ID of the applicant's pending application to the club,
//!   which stops a second one.
//! - `discord:apply-day:{hash}`: how many applications the applicant sent in the last day.
//!
//! The application and its marker expire with the application. The two lists are pruned when read.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::aio::ConnectionManager;
use redis::Script;
use tokio::time::timeout;

use crate::calendar::add_month;
use crate::config::KeySecret;
use crate::sealed::Sealer;
use crate::store::StoreError;
use crate::token::random;
use crate::trainer_store::applicant_hash;

const APP_PREFIX: &str = "discord:app:";
const CLUB_PREFIX: &str = "discord:club-apps:";
const USER_PREFIX: &str = "discord:my-apps:";
const APPLIED_PREFIX: &str = "discord:applied:";
const DAY_PREFIX: &str = "discord:apply-day:";
const LIMIT: Duration = Duration::from_secs(5);

/// Most applications one person can send in a day.
pub const MAX_PER_DAY: u32 = 5;

/// How long the lists keep their keys after the last application, a little over the longest
/// application's life.
const LIST_TTL: u64 = 32 * 24 * 60 * 60;

/// Saves an application unless the applicant has one pending to the club or is out of
/// applications for the day. KEYS are the application, the club's list, the applicant's list, the
/// marker and the day's counter; ARGV the ID, when it was sent, when it expires, the most a day,
/// the list TTL and then pairs of field and value. Returns 0 if saved, 1 for a pending one and 2
/// for the daily limit.
const CREATE: &str = r"
if redis.call('EXISTS', KEYS[4]) == 1 then
  return 1
end
if tonumber(redis.call('GET', KEYS[5]) or '0') >= tonumber(ARGV[4]) then
  return 2
end
for i = 6, #ARGV, 2 do
  redis.call('HSET', KEYS[1], ARGV[i], ARGV[i + 1])
end
redis.call('EXPIREAT', KEYS[1], ARGV[3])
redis.call('ZADD', KEYS[2], ARGV[2], ARGV[1])
redis.call('EXPIRE', KEYS[2], ARGV[5])
redis.call('ZADD', KEYS[3], ARGV[2], ARGV[1])
redis.call('EXPIRE', KEYS[3], ARGV[5])
redis.call('SET', KEYS[4], ARGV[1])
redis.call('EXPIREAT', KEYS[4], ARGV[3])
if redis.call('INCR', KEYS[5]) == 1 then
  redis.call('EXPIRE', KEYS[5], 86400)
end
return 0
";

/// Takes back an application that couldn't be finished, as if it was never sent. KEYS are the
/// application, the club's list, the applicant's list, the marker and the day's counter; ARGV the
/// ID.
const DISCARD: &str = r"
if redis.call('EXISTS', KEYS[1]) == 0 then
  return 0
end
redis.call('DEL', KEYS[1], KEYS[4])
redis.call('ZREM', KEYS[2], ARGV[1])
redis.call('ZREM', KEYS[3], ARGV[1])
if tonumber(redis.call('GET', KEYS[5]) or '0') > 0 then
  redis.call('DECR', KEYS[5])
end
return 1
";

/// Answers a pending application, and frees the applicant to apply to the club again. KEYS is the
/// application; ARGV the status and the marker prefix. Returns 1 if it was pending.
const ANSWER: &str = r"
if redis.call('HGET', KEYS[1], 'status') ~= 'pending' then
  return 0
end
redis.call('HSET', KEYS[1], 'status', ARGV[1])
local hash, club = redis.call('HGET', KEYS[1], 'h'), redis.call('HGET', KEYS[1], 'club')
redis.call('DEL', ARGV[2] .. hash .. ':' .. club)
return 1
";

/// Notes where an application's ticket is, if the application is still there.
const ATTACH: &str = r"
if redis.call('EXISTS', KEYS[1]) == 0 then
  return 0
end
redis.call('HSET', KEYS[1], 'channel', ARGV[1], 'message', ARGV[2])
return 1
";

/// Where an application stands with the club's leader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Waiting for an answer.
    Pending,
    /// The leader accepted it.
    Accepted,
    /// The leader declined it.
    Declined,
}

impl Status {
    /// The status as the API and the store spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Declined => "declined",
        }
    }

    fn parse(status: &str) -> Option<Self> {
        match status {
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            _ => None,
        }
    }
}

/// What an application is sent with.
#[derive(Debug, Clone, PartialEq)]
pub struct NewApplication {
    /// The club applied to.
    pub club: u64,
    /// That club's name.
    pub club_name: String,
    /// The Discord server of the club.
    pub guild: u64,
    /// The applicant's Discord ID.
    pub applicant: u64,
    /// The trainer ID they apply with.
    pub trainer: u64,
    /// That trainer's name.
    pub trainer_name: String,
    /// The name of the club they're in, if any.
    pub current_club: Option<String>,
    /// Fans per day.
    pub pace: Option<f64>,
    /// Fans gained this month.
    pub fans_this_month: Option<i64>,
}

/// An application as kept.
#[derive(Debug, Clone, PartialEq)]
pub struct Application {
    /// The application's ID.
    pub id: String,
    /// The details it was sent with.
    pub details: NewApplication,
    /// Where it stands.
    pub status: Status,
    /// When it was sent, in Unix seconds.
    pub submitted_at: u64,
    /// The ticket channel in the club's server, once there is one.
    pub channel: Option<u64>,
    /// The message in it with the answer buttons.
    pub message: Option<u64>,
}

/// What sending an application did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// It's saved, with this ID.
    Saved(String),
    /// The applicant already has one pending to the club.
    Pending,
    /// The applicant sent [`MAX_PER_DAY`] applications in the last day.
    OverLimit,
}

/// Applications to clubs. Cheap to clone.
#[derive(Clone)]
pub struct ApplicationStore {
    conn: ConnectionManager,
    secret: KeySecret,
    sealer: Sealer,
}

impl std::fmt::Debug for ApplicationStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicationStore").finish_non_exhaustive()
    }
}

impl ApplicationStore {
    /// A store using this connection, hashing Discord IDs with `secret`.
    pub fn new(conn: ConnectionManager, secret: KeySecret) -> Self {
        let sealer = Sealer::new(&secret);
        Self {
            conn,
            secret,
            sealer,
        }
    }

    /// Saves an application sent now, unless the applicant has a pending one to the club or has
    /// sent [`MAX_PER_DAY`] in the last day. It expires a calendar month later.
    pub async fn create(&self, application: &NewApplication) -> Result<Sent, StoreError> {
        let id = random();
        let hash = applicant_hash(&self.secret, application.applicant);
        let now = unix_now();
        let mut fields = vec![
            ("club", application.club.to_string()),
            ("club_name", application.club_name.clone()),
            ("guild", application.guild.to_string()),
            (
                "applicant",
                self.sealer.seal(&application.applicant.to_string(), &id),
            ),
            ("h", hash.clone()),
            ("trainer", application.trainer.to_string()),
            ("trainer_name", application.trainer_name.clone()),
            ("status", Status::Pending.as_str().to_owned()),
            ("submitted", now.to_string()),
        ];
        let optional = [
            ("current_club", application.current_club.clone()),
            ("pace", application.pace.map(|pace| pace.to_string())),
            (
                "fans",
                application.fans_this_month.map(|fans| fans.to_string()),
            ),
        ];
        fields.extend(
            optional
                .into_iter()
                .filter_map(|(field, value)| Some((field, value?))),
        );
        let script = Script::new(CREATE);
        let mut call = script.key(app_key(&id));
        call.key(club_key(application.club))
            .key(user_key(&hash))
            .key(applied_key(&hash, application.club))
            .key(format!("{DAY_PREFIX}{hash}"))
            .arg(&id)
            .arg(now)
            .arg(add_month(now))
            .arg(MAX_PER_DAY)
            .arg(LIST_TTL);
        for (field, value) in fields {
            call.arg(field).arg(value);
        }
        let reply: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        Ok(match reply {
            0 => Sent::Saved(id),
            1 => Sent::Pending,
            _ => Sent::OverLimit,
        })
    }

    /// Takes back an application that couldn't be finished, so it doesn't count.
    pub async fn discard(&self, application: &Application) -> Result<(), StoreError> {
        let details = &application.details;
        let hash = applicant_hash(&self.secret, details.applicant);
        let script = Script::new(DISCARD);
        let mut call = script.key(app_key(&application.id));
        call.key(club_key(details.club))
            .key(user_key(&hash))
            .key(applied_key(&hash, details.club))
            .key(format!("{DAY_PREFIX}{hash}"))
            .arg(&application.id);
        let _: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        Ok(())
    }

    /// Notes the ticket channel and its message. Returns whether the application still exists.
    pub async fn attach(&self, id: &str, channel: u64, message: u64) -> Result<bool, StoreError> {
        let script = Script::new(ATTACH);
        let mut call = script.key(app_key(id));
        call.arg(channel).arg(message);
        let found: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        Ok(found == 1)
    }

    /// Answers a pending application. Returns whether it was pending; an answered or expired one
    /// stays as it is.
    pub async fn answer(&self, id: &str, status: Status) -> Result<bool, StoreError> {
        let script = Script::new(ANSWER);
        let mut call = script.key(app_key(id));
        call.arg(status.as_str()).arg(APPLIED_PREFIX);
        let changed: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        Ok(changed == 1)
    }

    /// The application with this ID, or `None` if it doesn't exist or has expired.
    pub async fn get(&self, id: &str) -> Result<Option<Application>, StoreError> {
        let fields: HashMap<String, String> = self
            .run(
                redis::cmd("HGETALL")
                    .arg(app_key(id))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(parse(&self.sealer, id, &fields))
    }

    /// A club's applications, newest first.
    pub async fn for_club(&self, club: u64) -> Result<Vec<Application>, StoreError> {
        self.list(club_key(club)).await
    }

    /// An applicant's applications, newest first.
    pub async fn for_applicant(&self, user: u64) -> Result<Vec<Application>, StoreError> {
        self.list(user_key(&applicant_hash(&self.secret, user)))
            .await
    }

    /// Reads the application IDs in a list, newest first, dropping the ones that expired.
    async fn list(&self, key: String) -> Result<Vec<Application>, StoreError> {
        let ids: Vec<String> = self
            .run(
                redis::cmd("ZREVRANGE")
                    .arg(&key)
                    .arg(0)
                    .arg(-1)
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut pipeline = redis::pipe();
        for id in &ids {
            pipeline.cmd("HGETALL").arg(app_key(id));
        }
        let rows: Vec<HashMap<String, String>> = self
            .run(pipeline.query_async(&mut self.conn.clone()))
            .await?;
        let mut found = Vec::new();
        let mut expired = Vec::new();
        for (id, fields) in ids.iter().zip(&rows) {
            match parse(&self.sealer, id, fields) {
                Some(application) => found.push(application),
                // Only a record that's gone has expired; one that can't be read stays.
                None if fields.is_empty() => expired.push(id.as_str()),
                None => {}
            }
        }
        if !expired.is_empty() {
            let _: i64 = self
                .run(
                    redis::cmd("ZREM")
                        .arg(&key)
                        .arg(&expired)
                        .query_async(&mut self.conn.clone()),
                )
                .await?;
        }
        Ok(found)
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

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn app_key(id: &str) -> String {
    format!("{APP_PREFIX}{id}")
}

fn club_key(club: u64) -> String {
    format!("{CLUB_PREFIX}{club}")
}

fn user_key(hash: &str) -> String {
    format!("{USER_PREFIX}{hash}")
}

fn applied_key(hash: &str, club: u64) -> String {
    format!("{APPLIED_PREFIX}{hash}:{club}")
}

fn parse(sealer: &Sealer, id: &str, fields: &HashMap<String, String>) -> Option<Application> {
    let number = |field: &str| fields.get(field)?.parse::<u64>().ok();
    let text = |field: &str| fields.get(field).filter(|v| !v.is_empty()).cloned();
    Some(Application {
        id: id.to_owned(),
        details: NewApplication {
            club: number("club")?,
            club_name: text("club_name")?,
            guild: number("guild")?,
            applicant: sealer.open(fields.get("applicant")?, id)?.parse().ok()?,
            trainer: number("trainer")?,
            trainer_name: text("trainer_name")?,
            current_club: text("current_club"),
            pace: fields.get("pace").and_then(|v| v.parse().ok()),
            fans_this_month: fields.get("fans").and_then(|v| v.parse().ok()),
        },
        status: Status::parse(fields.get("status")?)?,
        submitted_at: number("submitted")?,
        channel: number("channel"),
        message: number("message"),
    })
}
