//! Ads trainers post to say they're looking for a club, kept in Redis.
//!
//! - `discord:ad:{hash}`: a hash with the ad's fields and one `inv:{club_id}` field, holding the
//!   time it was sent, for each club that invited the trainer. `{hash}` is a keyed hash of the
//!   trainer's Discord ID, so a person has one ad.
//! - `discord:ads`: the hashes of the ads on the board, scored by when each leaves it.
//!
//! An ad is on the board for [`VISIBLE_FOR`] after it's posted or renewed, and kept for
//! [`STORED_FOR`], so its trainer can bring it back. The ad and its invites expire together.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::aio::ConnectionManager;
use redis::Script;
use tokio::time::timeout;

use crate::config::KeySecret;
use crate::sealed::Sealer;
use crate::store::StoreError;
use crate::trainer_store::applicant_hash;

const AD_PREFIX: &str = "discord:ad:";
const BOARD: &str = "discord:ads";
const INVITE_FIELD: &str = "inv:";
const LIMIT: Duration = Duration::from_secs(5);

/// How long an ad stays on the board after it's posted or renewed.
pub const VISIBLE_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// How long an ad is kept after it's posted or renewed, so it can be renewed once it's left the
/// board.
pub const STORED_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Most clubs that can invite one ad's trainer at a time.
pub const MAX_INVITES: usize = 10;

/// Most ads the board lists.
pub const MAX_ON_BOARD: usize = 500;

/// The lowest and highest club rank, as tier codes from D to SS.
pub const RANKS: std::ops::RangeInclusive<u8> = 1..=11;

/// Most club ranks one ad can ask for, both ends counted: C to B (C, C+ and B) is three.
pub const MAX_RANK_SPAN: u8 = 3;

/// Saves an ad, keeping its invites and when it was first posted. KEYS are the ad and the board;
/// ARGV the ID, now, when it leaves the board, when it's deleted and then pairs of field and
/// value, where an empty value removes the field.
const SAVE: &str = r"
for i = 5, #ARGV, 2 do
  if ARGV[i + 1] == '' then
    redis.call('HDEL', KEYS[1], ARGV[i])
  else
    redis.call('HSET', KEYS[1], ARGV[i], ARGV[i + 1])
  end
end
redis.call('HSETNX', KEYS[1], 'created', ARGV[2])
redis.call('HSET', KEYS[1], 'visible_until', ARGV[3])
redis.call('EXPIREAT', KEYS[1], ARGV[4])
redis.call('ZADD', KEYS[2], ARGV[3], ARGV[1])
return 1
";

/// Notes that a club invited the ad's trainer. KEYS is the ad; ARGV the club, now and the most
/// invites. Returns 0 if invited, 1 for no such ad on the board, 2 for the most invites and 3 for
/// a club that already invited them.
const INVITE: &str = r"
if redis.call('EXISTS', KEYS[1]) == 0 then
  return 1
end
if tonumber(redis.call('HGET', KEYS[1], 'visible_until') or '0') <= tonumber(ARGV[2]) then
  return 1
end
if redis.call('HEXISTS', KEYS[1], 'inv:' .. ARGV[1]) == 1 then
  return 3
end
local count = 0
for _, field in ipairs(redis.call('HKEYS', KEYS[1])) do
  if string.sub(field, 1, 4) == 'inv:' then
    count = count + 1
  end
end
if count >= tonumber(ARGV[3]) then
  return 2
end
redis.call('HSET', KEYS[1], 'inv:' .. ARGV[1], ARGV[2])
return 0
";

/// What a trainer wants from a club's Discord server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordWish {
    /// The club must have one.
    Required,
    /// They'd like one.
    Preferred,
    /// They don't mind.
    Any,
}

impl DiscordWish {
    /// The wish as the API and the store spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Preferred => "preferred",
            Self::Any => "any",
        }
    }

    /// The wish spelled `text`, if it is one.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "required" => Some(Self::Required),
            "preferred" => Some(Self::Preferred),
            "any" => Some(Self::Any),
            _ => None,
        }
    }
}

/// The language a trainer wants their club to speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// English.
    English,
    /// Japanese.
    Japanese,
    /// Korean.
    Korean,
    /// Chinese.
    Chinese,
    /// Spanish.
    Spanish,
    /// Portuguese.
    Portuguese,
    /// French.
    French,
    /// German.
    German,
    /// Indonesian.
    Indonesian,
    /// Thai.
    Thai,
    /// Vietnamese.
    Vietnamese,
    /// They don't mind.
    Any,
}

impl Language {
    /// Every language, `Any` last.
    pub const ALL: [Self; 12] = [
        Self::English,
        Self::Japanese,
        Self::Korean,
        Self::Chinese,
        Self::Spanish,
        Self::Portuguese,
        Self::French,
        Self::German,
        Self::Indonesian,
        Self::Thai,
        Self::Vietnamese,
        Self::Any,
    ];

    /// The language as the API and the store spell it: its two-letter code, or `any`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Chinese => "zh",
            Self::Spanish => "es",
            Self::Portuguese => "pt",
            Self::French => "fr",
            Self::German => "de",
            Self::Indonesian => "id",
            Self::Thai => "th",
            Self::Vietnamese => "vi",
            Self::Any => "any",
        }
    }

    /// The language spelled `text`, if it is one.
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.as_str() == text)
    }
}

/// What an ad is posted with.
#[derive(Debug, Clone, PartialEq)]
pub struct NewAd {
    /// The trainer's Discord ID.
    pub user: u64,
    /// The trainer ID the ad is for.
    pub trainer: u64,
    /// That trainer's name.
    pub trainer_name: String,
    /// The name of the club they're in, if any.
    pub current_club: Option<String>,
    /// Fans per day.
    pub pace: Option<f64>,
    /// Fans gained this month.
    pub fans_this_month: Option<i64>,
    /// The outfit ID of their leader character.
    pub leader_chara_dress_id: Option<i64>,
    /// The lowest club rank they'd join, as a tier code in [`RANKS`].
    pub rank_min: u8,
    /// The highest club rank they'd join, as a tier code in [`RANKS`].
    pub rank_max: u8,
    /// Whether they want the club to have a Discord server.
    pub discord: DiscordWish,
    /// The language they want the club to speak.
    pub language: Language,
}

/// A club's invite to the ad's trainer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invite {
    /// The club that invited them.
    pub club: u64,
    /// When it was sent, in Unix seconds.
    pub sent_at: u64,
}

/// An ad as kept.
#[derive(Debug, Clone, PartialEq)]
pub struct Ad {
    /// The ad's ID, which leaders use to invite its trainer.
    pub id: String,
    /// What it was posted with.
    pub details: NewAd,
    /// When it was first posted, in Unix seconds.
    pub created_at: u64,
    /// When it leaves the board unless renewed, in Unix seconds.
    pub visible_until: u64,
    /// Clubs that invited the trainer, newest first.
    pub invites: Vec<Invite>,
}

/// What inviting a trainer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invited {
    /// The invite is saved.
    Sent,
    /// The ad isn't on the board.
    Gone,
    /// The trainer has [`MAX_INVITES`] invites.
    Full,
    /// The club already invited them.
    Already,
}

/// Ads for trainers looking for a club. Cheap to clone.
#[derive(Clone)]
pub struct AdStore {
    conn: ConnectionManager,
    secret: KeySecret,
    sealer: Sealer,
}

impl std::fmt::Debug for AdStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdStore").finish_non_exhaustive()
    }
}

impl AdStore {
    /// A store using this connection, hashing Discord IDs with `secret`.
    pub fn new(conn: ConnectionManager, secret: KeySecret) -> Self {
        let sealer = Sealer::new(&secret);
        Self {
            conn,
            secret,
            sealer,
        }
    }

    /// Posts the trainer's ad, or replaces their earlier one, keeping its invites. Either way it's
    /// on the board for [`VISIBLE_FOR`] from now.
    pub async fn save(&self, ad: &NewAd) -> Result<Ad, StoreError> {
        let id = applicant_hash(&self.secret, ad.user);
        let now = unix_now();
        let fields = [
            ("applicant", self.sealer.seal(&ad.user.to_string(), &id)),
            ("trainer", ad.trainer.to_string()),
            ("trainer_name", ad.trainer_name.clone()),
            ("current_club", ad.current_club.clone().unwrap_or_default()),
            ("pace", ad.pace.map(|v| v.to_string()).unwrap_or_default()),
            (
                "fans",
                ad.fans_this_month
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "dress",
                ad.leader_chara_dress_id
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            ("rank_min", ad.rank_min.to_string()),
            ("rank_max", ad.rank_max.to_string()),
            ("discord", ad.discord.as_str().to_owned()),
            ("language", ad.language.as_str().to_owned()),
        ];
        let script = Script::new(SAVE);
        let mut call = script.key(ad_key(&id));
        call.key(BOARD)
            .arg(&id)
            .arg(now)
            .arg(now + VISIBLE_FOR.as_secs())
            .arg(now + STORED_FOR.as_secs());
        for (field, value) in fields {
            call.arg(field).arg(value);
        }
        let _: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        self.get(ad.user)
            .await?
            .ok_or_else(|| StoreError::Redis("the ad was not found after it was saved".into()))
    }

    /// The trainer's ad, on the board or not, or `None` once it has been deleted or has expired.
    pub async fn get(&self, user: u64) -> Result<Option<Ad>, StoreError> {
        let id = applicant_hash(&self.secret, user);
        let fields: HashMap<String, String> = self
            .run(
                redis::cmd("HGETALL")
                    .arg(ad_key(&id))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(parse(&self.sealer, &id, &fields))
    }

    /// The ads on the board, most recently renewed first.
    pub async fn board(&self) -> Result<Vec<Ad>, StoreError> {
        let now = unix_now();
        let ids: Vec<String> = self
            .run(
                redis::cmd("ZREVRANGEBYSCORE")
                    .arg(BOARD)
                    .arg("+inf")
                    .arg(format!("({now}"))
                    .arg("LIMIT")
                    .arg(0)
                    .arg(MAX_ON_BOARD)
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        // Ads that left the board stay stored for a renewal but aren't listed.
        let _: i64 = self
            .run(
                redis::cmd("ZREMRANGEBYSCORE")
                    .arg(BOARD)
                    .arg("-inf")
                    .arg(now)
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut pipeline = redis::pipe();
        for id in &ids {
            pipeline.cmd("HGETALL").arg(ad_key(id));
        }
        let rows: Vec<HashMap<String, String>> = self
            .run(pipeline.query_async(&mut self.conn.clone()))
            .await?;
        let mut found = Vec::new();
        let mut gone = Vec::new();
        for (id, fields) in ids.iter().zip(&rows) {
            match parse(&self.sealer, id, fields) {
                Some(ad) => found.push(ad),
                // Only an ad that's gone is dropped; one that can't be read stays.
                None if fields.is_empty() => gone.push(id.as_str()),
                None => {}
            }
        }
        if !gone.is_empty() {
            let _: i64 = self
                .run(
                    redis::cmd("ZREM")
                        .arg(BOARD)
                        .arg(&gone)
                        .query_async(&mut self.conn.clone()),
                )
                .await?;
        }
        Ok(found)
    }

    /// Notes that `club` invited the trainer whose ad has this ID.
    pub async fn invite(&self, id: &str, club: u64) -> Result<Invited, StoreError> {
        let script = Script::new(INVITE);
        let mut call = script.key(ad_key(id));
        call.arg(club).arg(unix_now()).arg(MAX_INVITES);
        let reply: i64 = self.run(call.invoke_async(&mut self.conn.clone())).await?;
        Ok(match reply {
            0 => Invited::Sent,
            1 => Invited::Gone,
            2 => Invited::Full,
            _ => Invited::Already,
        })
    }

    /// Removes `club`'s invite from the trainer's ad. Returns whether there was one.
    pub async fn dismiss(&self, user: u64, club: u64) -> Result<bool, StoreError> {
        let id = applicant_hash(&self.secret, user);
        let removed: i64 = self
            .run(
                redis::cmd("HDEL")
                    .arg(ad_key(&id))
                    .arg(format!("{INVITE_FIELD}{club}"))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(removed == 1)
    }

    /// Deletes the trainer's ad and its invites. Returns whether there was one.
    pub async fn delete(&self, user: u64) -> Result<bool, StoreError> {
        let id = applicant_hash(&self.secret, user);
        let mut pipeline = redis::pipe();
        pipeline
            .cmd("DEL")
            .arg(ad_key(&id))
            .cmd("ZREM")
            .arg(BOARD)
            .arg(&id);
        let (deleted, _): (i64, i64) = self
            .run(pipeline.query_async(&mut self.conn.clone()))
            .await?;
        Ok(deleted == 1)
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

fn ad_key(id: &str) -> String {
    format!("{AD_PREFIX}{id}")
}

fn parse(sealer: &Sealer, id: &str, fields: &HashMap<String, String>) -> Option<Ad> {
    let number = |field: &str| fields.get(field)?.parse::<u64>().ok();
    let text = |field: &str| fields.get(field).filter(|v| !v.is_empty()).cloned();
    let rank = |field: &str| {
        number(field)
            .and_then(|v| u8::try_from(v).ok())
            .filter(|v| RANKS.contains(v))
    };
    let (rank_min, rank_max) = (rank("rank_min")?, rank("rank_max")?);
    if rank_min > rank_max {
        return None;
    }
    let mut invites: Vec<Invite> = fields
        .iter()
        .filter_map(|(field, sent)| {
            Some(Invite {
                club: field.strip_prefix(INVITE_FIELD)?.parse().ok()?,
                sent_at: sent.parse().ok()?,
            })
        })
        .collect();
    invites.sort_by(|a, b| b.sent_at.cmp(&a.sent_at).then(a.club.cmp(&b.club)));
    Some(Ad {
        id: id.to_owned(),
        details: NewAd {
            user: sealer.open(fields.get("applicant")?, id)?.parse().ok()?,
            trainer: number("trainer")?,
            trainer_name: text("trainer_name")?,
            current_club: text("current_club"),
            pace: fields.get("pace").and_then(|v| v.parse().ok()),
            fans_this_month: fields.get("fans").and_then(|v| v.parse().ok()),
            leader_chara_dress_id: fields.get("dress").and_then(|v| v.parse().ok()),
            rank_min,
            rank_max,
            discord: DiscordWish::parse(fields.get("discord")?)?,
            language: Language::parse(fields.get("language")?)?,
        },
        created_at: number("created")?,
        visible_until: number("visible_until")?,
        invites,
    })
}

#[cfg(test)]
mod tests;
