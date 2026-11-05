//! Linking a trainer ID to a Discord user: the user puts a key we give them in their in-game
//! profile comment, and the link is made once uma.moe shows the key there.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::extract::{Path, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::{error, private, session_user};
use crate::bot::Discord;
use crate::opt_in::{store_unavailable, LookupError};
use crate::store::StoreError;
use crate::token::seconds;
use crate::trainer_store::{
    Action, CheckState, DueCheck, LinkOutcome, PendingLink, TrainerStore, MAX_LINKED_TRAINERS,
};

/// Path that lists the signed-in user's linked trainer IDs. `GET`.
pub const TRAINERS_PATH: &str = "/api/v1/discord/trainers";

/// Path of one trainer ID, with `{trainer}` the ID. `DELETE` unlinks it from the signed-in user.
pub const TRAINER_PATH: &str = "/api/v1/discord/trainers/{trainer}";

/// Path that starts linking a trainer ID, answering with the key for the profile comment. `POST`;
/// `DELETE` forgets the key and stops any check for it.
pub const VERIFICATION_PATH: &str = "/api/v1/discord/trainers/{trainer}/verification";

/// Path that checks the trainer's profile for the key, links the ID if it's there, and otherwise
/// keeps checking in the background. `POST`.
pub const LINK_PATH: &str = "/api/v1/discord/trainers/{trainer}/link";

/// Letters and digits that can't be mistaken for each other: no `I`, `L`, `O`, `0` or `1`.
const ALPHABET: &[u8; 31] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const KEY_PREFIX: &str = "uma-";
const KEY_CHARS: usize = 8;
/// New keys to try before giving up, in case one is already taken.
const KEY_TRIES: usize = 5;

/// How long a check keeps looking for the key after the first try.
pub const CHECK_FOR: Duration = Duration::from_secs(5 * 60);

/// How often a running check looks again.
pub const CHECK_EVERY: Duration = Duration::from_secs(60);

/// How often the background task looks for checks that are due.
const CHECK_POLL: Duration = Duration::from_secs(5);

/// What linking needs to know about a trainer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerSummary {
    /// The trainer's name.
    pub name: String,
    /// The profile comment, if they have one.
    pub comment: Option<String>,
}

type Lookup = dyn Fn(u64) -> Pin<Box<dyn Future<Output = Result<Option<TrainerSummary>, LookupError>> + Send>>
    + Send
    + Sync;

/// Looks trainers up by ID, fresh each time. Cheap to clone.
#[derive(Clone)]
pub struct TrainerLookup(Arc<Lookup>);

impl TrainerLookup {
    /// A lookup that calls `lookup`, which answers `Ok(None)` for a trainer that doesn't exist.
    pub fn new<F, Fut>(lookup: F) -> Self
    where
        F: Fn(u64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<TrainerSummary>, LookupError>> + Send + 'static,
    {
        Self(Arc::new(move |id| Box::pin(lookup(id))))
    }

    /// The trainer with this ID: `None` if there is none.
    pub async fn get(&self, trainer_id: u64) -> Result<Option<TrainerSummary>, LookupError> {
        (self.0)(trainer_id).await
    }
}

impl std::fmt::Debug for TrainerLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainerLookup").finish_non_exhaustive()
    }
}

/// What linking trainer IDs uses: where links are kept and where profiles come from.
#[derive(Debug, Clone)]
pub struct TrainerLinks {
    store: TrainerStore,
    trainers: TrainerLookup,
}

impl TrainerLinks {
    /// Trainer links kept in `store`, checking profiles with `trainers`.
    pub fn new(store: TrainerStore, trainers: TrainerLookup) -> Self {
        Self { store, trainers }
    }

    /// Where links are kept.
    pub fn store(&self) -> &TrainerStore {
        &self.store
    }

    /// Runs the background checks, looking for due ones every few seconds. Spawn it once per
    /// server; several servers can share the store.
    pub async fn run_checks(self) {
        let mut tick = tokio::time::interval(CHECK_POLL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if let Err(err) = self.check_due(seconds(SystemTime::now())).await {
                tracing::warn!(error = %err, "couldn't run the trainer link checks");
            }
        }
    }

    /// Tries every check due by `now` (Unix seconds) once. Returns how many it tried.
    pub async fn check_due(&self, now: u64) -> Result<usize, StoreError> {
        let due = self.store.claim_due(now, CHECK_EVERY.as_secs()).await?;
        for check in &due {
            self.retry(check, now).await?;
        }
        Ok(due.len())
    }

    async fn retry(&self, check: &DueCheck, now: u64) -> Result<(), StoreError> {
        let Some(until) = self.store.running_until(check).await? else {
            return self.store.finish(check, None).await;
        };
        match attempt(self, &check.hash, check.trainer).await? {
            Attempt::Linked(_) | Attempt::NoKey => self.store.finish(check, None).await,
            Attempt::NotFound => self.store.finish(check, Some(CheckState::NotFound)).await,
            Attempt::TooMany => self.store.finish(check, Some(CheckState::TooMany)).await,
            Attempt::Missing | Attempt::Unavailable if now < until => {
                self.store
                    .retry_at(check, now + CHECK_EVERY.as_secs())
                    .await
            }
            Attempt::Missing | Attempt::Unavailable => {
                count("gave_up");
                self.store.finish(check, Some(CheckState::Missing)).await
            }
        }
    }
}

/// Serves the trainer-link routes, if they're set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord.trainer_links().is_none() {
        return Router::new();
    }
    Router::new()
        .route(TRAINERS_PATH, get(list))
        .route(TRAINER_PATH, delete(unlink))
        .route(VERIFICATION_PATH, post(start).delete(discard))
        .route(LINK_PATH, post(check))
        .with_state(discord)
}

async fn list(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let linked = match links.store.linked_trainers(user).await {
        Ok(linked) => linked,
        Err(err) => return store_unavailable(&err),
    };
    let pending = match links.store.pending_links(user).await {
        Ok(pending) => pending,
        Err(err) => return store_unavailable(&err),
    };
    let now = seconds(SystemTime::now());
    let linked: Vec<_> = linked
        .iter()
        .map(|t| json!({ "id": t.trainer.to_string(), "name": t.name }))
        .collect();
    let pending: Vec<_> = pending.iter().map(|p| pending_json(p, now)).collect();
    private(Json(json!({
        "linked": linked,
        "max": MAX_LINKED_TRAINERS,
        "pending": pending,
    })))
}

fn pending_json(link: &PendingLink, now: u64) -> serde_json::Value {
    let check = link.check.as_ref().map(|check| match check {
        CheckState::Checking { until } => json!({
            "state": "checking",
            "seconds_left": until.saturating_sub(now),
        }),
        CheckState::Missing => json!({ "state": "missing" }),
        CheckState::NotFound => json!({ "state": "not_found" }),
        CheckState::TooMany => json!({ "state": "too_many" }),
    });
    json!({
        "trainer": link.trainer.to_string(),
        "key": link.pending.key,
        "expires_in": link.pending.expires_in,
        "check": check,
    })
}

async fn discard(
    State(discord): State<Discord>,
    Path(trainer): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(trainer) = parse_trainer(&trainer) else {
        return error(StatusCode::BAD_REQUEST, "trainer must be a trainer ID");
    };
    match links.store.discard(user, trainer).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => store_unavailable(&err),
    }
}

async fn start(
    State(discord): State<Discord>,
    Path(trainer): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(trainer) = parse_trainer(&trainer) else {
        return error(StatusCode::BAD_REQUEST, "trainer must be a trainer ID");
    };
    match links.store.linked(user).await {
        Ok(linked) if linked.contains(&trainer) => {
            return error(StatusCode::CONFLICT, "that trainer is already linked")
        }
        Ok(_) => {}
        Err(err) => return store_unavailable(&err),
    }
    if let Some(response) = limited(links, user, Action::Start).await {
        return response;
    }
    for _ in 0..KEY_TRIES {
        match links.store.start(user, trainer, &new_key()).await {
            Ok(Some(pending)) => {
                return private(Json(json!({
                    "key": pending.key,
                    "expires_in": pending.expires_in,
                })))
            }
            // Someone else holds that key: make another.
            Ok(None) => {}
            Err(err) => return store_unavailable(&err),
        }
    }
    tracing::error!("every new verification key was already taken");
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "couldn't make a key; try again",
    )
}

async fn check(
    State(discord): State<Discord>,
    Path(trainer): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(trainer) = parse_trainer(&trainer) else {
        return error(StatusCode::BAD_REQUEST, "trainer must be a trainer ID");
    };
    match links.store.pending(user, trainer).await {
        Ok(Some(_)) => {}
        Ok(None) => return no_key(),
        Err(err) => return store_unavailable(&err),
    }
    let now = seconds(SystemTime::now());
    // A check that's running answers for itself: no new try, and no count against the limit.
    match links.store.check_state(user, trainer).await {
        Ok(Some(CheckState::Checking { until })) => return checking(until, now),
        Ok(_) => {}
        Err(err) => return store_unavailable(&err),
    }
    if let Some(response) = limited(links, user, Action::Check).await {
        count("rate_limited");
        return response;
    }
    let attempt = match attempt(links, &links.store.hash(user), trainer).await {
        Ok(attempt) => attempt,
        Err(err) => return store_unavailable(&err),
    };
    match attempt {
        Attempt::Linked(name) => private(Json(json!({ "linked": true, "name": name }))),
        Attempt::NoKey => no_key(),
        Attempt::NotFound => error(StatusCode::NOT_FOUND, "trainer not found"),
        Attempt::TooMany => error(
            StatusCode::FORBIDDEN,
            "you've linked as many trainers as you can; unlink one first",
        ),
        Attempt::Missing | Attempt::Unavailable => {
            let until = now + CHECK_FOR.as_secs();
            let next = now + CHECK_EVERY.as_secs();
            match links.store.begin_check(user, trainer, until, next).await {
                Ok(_) => checking(until, now),
                Err(err) => store_unavailable(&err),
            }
        }
    }
}

fn no_key() -> Response {
    error(
        StatusCode::CONFLICT,
        "there's no key to look for; ask for one first",
    )
}

/// `202 Accepted` while a check runs, with the seconds it has left.
fn checking(until: u64, now: u64) -> Response {
    let body = json!({ "checking": true, "seconds_left": until.saturating_sub(now) });
    private((StatusCode::ACCEPTED, Json(body)))
}

/// What one look at a trainer's profile found.
enum Attempt {
    Linked(String),
    Missing,
    NotFound,
    TooMany,
    Unavailable,
    NoKey,
}

/// Looks for the pending key of the user hashed `hash` in `trainer`'s profile, linking it if the
/// key is there.
async fn attempt(links: &TrainerLinks, hash: &str, trainer: u64) -> Result<Attempt, StoreError> {
    let Some(key) = links.store.pending_for(hash, trainer).await? else {
        return Ok(Attempt::NoKey);
    };
    let profile = match links.trainers.get(trainer).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            count("not_found");
            return Ok(Attempt::NotFound);
        }
        Err(_) => {
            count("unavailable");
            return Ok(Attempt::Unavailable);
        }
    };
    if !profile
        .comment
        .as_deref()
        .is_some_and(|comment| contains_key(comment, &key))
    {
        count("mismatch");
        return Ok(Attempt::Missing);
    }
    Ok(
        match links.store.link_for(hash, trainer, &profile.name).await? {
            LinkOutcome::Linked => {
                count("linked");
                Attempt::Linked(profile.name)
            }
            LinkOutcome::TooMany => {
                count("too_many");
                Attempt::TooMany
            }
        },
    )
}

async fn unlink(
    State(discord): State<Discord>,
    Path(trainer): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(trainer) = parse_trainer(&trainer) else {
        return error(StatusCode::BAD_REQUEST, "trainer must be a trainer ID");
    };
    match links.store.unlink(user, trainer).await {
        Ok(true) => {
            metrics::counter!("discord_trainer_unlinks_total").increment(1);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => error(StatusCode::NOT_FOUND, "that trainer isn't linked to you"),
        Err(err) => store_unavailable(&err),
    }
}

/// The trainer links and the signed-in user, or the response to send.
fn signed_in<'a>(
    discord: &'a Discord,
    headers: &HeaderMap,
) -> Result<(&'a TrainerLinks, u64), Box<Response>> {
    let Some(links) = discord.trainer_links() else {
        return Err(Box::new(error(StatusCode::NOT_FOUND, "not found")));
    };
    match session_user(discord.config(), headers) {
        Some(user) => Ok((links, user)),
        None => Err(Box::new(error(
            StatusCode::UNAUTHORIZED,
            "sign in with Discord first",
        ))),
    }
}

/// The response for a user who has tried `action` too often, or a failed count; `None` if allowed.
async fn limited(links: &TrainerLinks, user: u64, action: Action) -> Option<Response> {
    match links.store.hit(user, action).await {
        Ok(None) => None,
        Ok(Some(wait)) => {
            let mut response = error(
                StatusCode::TOO_MANY_REQUESTS,
                "too many tries; wait a while and try again",
            );
            if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
                response.headers_mut().insert(RETRY_AFTER, value);
            }
            Some(response)
        }
        Err(err) => Some(store_unavailable(&err)),
    }
}

fn count(result: &'static str) {
    metrics::counter!("discord_trainer_links_total", "result" => result).increment(1);
}

/// The trainer ID in `text`: 9 to 12 digits.
fn parse_trainer(text: &str) -> Option<u64> {
    (9..=12)
        .contains(&text.len())
        .then(|| text.bytes().all(|b| b.is_ascii_digit()))
        .filter(|digits| *digits)
        .and_then(|_| text.parse().ok())
}

/// A new verification key, such as `uma-7KQ4XMPR`.
fn new_key() -> String {
    let mut key = String::from(KEY_PREFIX);
    let mut chars = 0;
    while chars < KEY_CHARS {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("the operating system's random number generator works");
        for byte in bytes {
            // Skip the top of the range, so every character is equally likely.
            if chars < KEY_CHARS && usize::from(byte) < 248 {
                key.push(char::from(ALPHABET[usize::from(byte) % ALPHABET.len()]));
                chars += 1;
            }
        }
    }
    key
}

/// Whether `comment` holds `key`, ignoring case, full-width forms and the kind of dash: the game
/// saves `-` as `－`.
fn contains_key(comment: &str, key: &str) -> bool {
    let fold = |text: &str| -> String { text.chars().map(plain).collect() };
    fold(comment).contains(&fold(key))
}

/// `c` as plain upper-case ASCII where it has a look-alike there.
fn plain(c: char) -> char {
    let c = match c {
        // Full-width ASCII, such as `Ａ` and `－`.
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(u32::from(c) - 0xFEE0).unwrap_or(c),
        // Hyphens, dashes, minus signs and the long vowel marks a keyboard may give instead.
        '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{FE63}' | '\u{30FC}' | '\u{FF70}' => '-',
        _ => c,
    };
    c.to_ascii_uppercase()
}

#[cfg(test)]
mod tests;
