//! Linking a trainer ID to a Discord user: the user puts a key we give them in their in-game
//! profile comment, and the link is made once uma.moe shows the key there.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
use crate::trainer_store::{Action, LinkOutcome, TrainerStore, MAX_LINKED_TRAINERS};

/// Path that lists the signed-in user's linked trainer IDs. `GET`.
pub const TRAINERS_PATH: &str = "/api/v1/discord/trainers";

/// Path of one trainer ID, with `{trainer}` the ID. `DELETE` unlinks it from the signed-in user.
pub const TRAINER_PATH: &str = "/api/v1/discord/trainers/{trainer}";

/// Path that starts linking a trainer ID, answering with the key for the profile comment. `POST`.
pub const VERIFICATION_PATH: &str = "/api/v1/discord/trainers/{trainer}/verification";

/// Path that checks the trainer's profile for the key and links the ID if it's there. `POST`.
pub const LINK_PATH: &str = "/api/v1/discord/trainers/{trainer}/link";

/// Letters and digits that can't be mistaken for each other: no `I`, `L`, `O`, `0` or `1`.
const ALPHABET: &[u8; 31] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const KEY_PREFIX: &str = "uma-";
const KEY_CHARS: usize = 6;

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
}

/// Serves the trainer-link routes, if they're set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord.trainer_links().is_none() {
        return Router::new();
    }
    Router::new()
        .route(TRAINERS_PATH, get(list))
        .route(TRAINER_PATH, delete(unlink))
        .route(VERIFICATION_PATH, post(start))
        .route(LINK_PATH, post(check))
        .with_state(discord)
}

async fn list(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let (links, user) = match signed_in(&discord, &headers) {
        Ok(found) => found,
        Err(response) => return *response,
    };
    match links.store.linked(user).await {
        Ok(trainers) => {
            let trainers: Vec<String> = trainers.iter().map(u64::to_string).collect();
            private(Json(
                json!({ "linked": trainers, "max": MAX_LINKED_TRAINERS }),
            ))
        }
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
    match links.store.start(user, trainer, &new_key()).await {
        Ok(pending) => private(Json(json!({
            "key": pending.key,
            "expires_in": pending.expires_in,
        }))),
        Err(err) => store_unavailable(&err),
    }
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
    let key = match links.store.pending(user, trainer).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            return error(
                StatusCode::CONFLICT,
                "there's no key to look for; ask for one first",
            )
        }
        Err(err) => return store_unavailable(&err),
    };
    if let Some(response) = limited(links, user, Action::Check).await {
        count("rate_limited");
        return response;
    }
    let profile = match links.trainers.get(trainer).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            count("not_found");
            return error(StatusCode::NOT_FOUND, "trainer not found");
        }
        Err(_) => {
            count("unavailable");
            return error(
                StatusCode::BAD_GATEWAY,
                "couldn't read the trainer's profile; try again",
            );
        }
    };
    if !profile
        .comment
        .as_deref()
        .is_some_and(|comment| contains_key(comment, &key))
    {
        count("mismatch");
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "the key isn't in the profile comment yet; it can take a while to show up",
        );
    }
    match links.store.link(user, trainer).await {
        Ok(LinkOutcome::Linked) => {
            count("linked");
            private(Json(json!({ "linked": true, "name": profile.name })))
        }
        Ok(LinkOutcome::TooMany) => {
            count("too_many");
            error(
                StatusCode::FORBIDDEN,
                "you've linked as many trainers as you can; unlink one first",
            )
        }
        Err(err) => store_unavailable(&err),
    }
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

/// A new verification key, such as `uma-7KQ4XM`.
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

/// Whether `comment` holds `key`, ignoring case.
fn contains_key(comment: &str, key: &str) -> bool {
    comment
        .to_ascii_uppercase()
        .contains(&key.to_ascii_uppercase())
}

#[cfg(test)]
mod tests;
