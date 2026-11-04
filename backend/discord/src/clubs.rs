//! A leader's clubs: what access the signed-in person has to a club, and the clubs on their
//! server they can see.

use std::time::{Duration, SystemTime};

use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{cookie_values, error, session_user, set_cookie};
use crate::bot::Discord;
use crate::config::DiscordConfig;
use crate::guild_access::rights;
use crate::opt_in::{member_club, store_unavailable};
use crate::token::{open, seal, seconds};

/// Path that lists the clubs on the signed-in person's server that they can see. `GET`.
pub const CLUBS_PATH: &str = "/api/v1/discord/clubs";

/// How long the browser remembers a person's server, so the list can be found again after signing
/// in again.
pub const MAX_HINT_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

const HINT_PURPOSE: &str = "guild";
const HINT_COOKIE: &str = "uma_guild";

/// The server a person last had access to a club on. It only says where to look: every list
/// checks the person's rights with Discord again.
#[derive(Serialize, Deserialize)]
struct Hint {
    /// The person it was made for, so it can't be used by someone else on the same browser.
    u: u64,
    /// The server.
    g: u64,
}

pub(crate) fn router(discord: Discord) -> Router {
    Router::new()
        .route(CLUBS_PATH, get(list))
        .with_state(discord)
}

/// What the signed-in person can do with one club, and the browser is told to remember its server.
/// Needs Manage Server on the club's server, or one of the club's leader roles.
pub(crate) async fn access(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (user, club_id, record, rights) = match member_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(level) = rights.level(&record.leader_roles) else {
        metrics::counter!("discord_club_access_total", "result" => "forbidden").increment(1);
        return error(
            StatusCode::FORBIDDEN,
            "you need Manage Server or one of the club's leader roles for this",
        );
    };
    metrics::counter!("discord_club_access_total", "result" => level.as_str()).increment(1);
    let mut response = private(Json(json!({
        "club": club_id.to_string(),
        "guild_id": record.guild_id.to_string(),
        "access": level.as_str(),
    })));
    response.headers_mut().append(
        SET_COOKIE,
        hint_cookie(discord.config(), user, record.guild_id),
    );
    response
}

/// The clubs on the person's remembered server that they can see, with their access to each. With
/// no remembered server, or no rights there any more, the list is empty.
async fn list(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let (Some(opt_in), config) = (discord.opt_in(), discord.config()) else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    let Some(user) = session_user(config, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "sign in with Discord first");
    };
    let none = || private(Json(json!({ "guild_id": null, "clubs": [] })));
    let Some(guild_id) = hinted_guild(config, &headers, user) else {
        return none();
    };
    let rights = match rights(discord.rest(), guild_id, user).await {
        Ok(Some(rights)) => rights,
        Ok(None) => return none(),
        Err(err)
            if matches!(
                err.status(),
                Some(StatusCode::FORBIDDEN | StatusCode::NOT_FOUND)
            ) =>
        {
            return none()
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not check who manages a Discord server");
            return error(
                StatusCode::BAD_GATEWAY,
                "couldn't check your rights on the server; try again",
            );
        }
    };
    let ids = match opt_in.store().clubs_on(guild_id).await {
        Ok(ids) => ids,
        Err(err) => return store_unavailable(&err),
    };
    let mut clubs = Vec::new();
    for id in ids {
        match opt_in.store().club(id).await {
            Ok(Some(record)) => {
                // A club that moved while this ran isn't on this server any more.
                if record.guild_id != guild_id {
                    continue;
                }
                if let Some(level) = rights.level(&record.leader_roles) {
                    clubs.push(json!({ "id": id.to_string(), "access": level.as_str() }));
                }
            }
            Ok(None) => {}
            Err(err) => return store_unavailable(&err),
        }
    }
    private(Json(
        json!({ "guild_id": guild_id.to_string(), "clubs": clubs }),
    ))
}

/// A response that must never be cached, as it holds a person's access.
fn private(body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The cookie that remembers `guild_id` as `user`'s server.
fn hint_cookie(config: &DiscordConfig, user: u64, guild_id: u64) -> HeaderValue {
    let hint = Hint {
        u: user,
        g: guild_id,
    };
    let expires = seconds(SystemTime::now() + MAX_HINT_AGE);
    let token = seal(&config.signing_key, HINT_PURPOSE, &hint, expires);
    set_cookie(config, HINT_COOKIE, &token, "/", MAX_HINT_AGE)
}

/// The cookie that makes the browser forget the remembered server.
pub(crate) fn clear_hint(config: &DiscordConfig) -> HeaderValue {
    set_cookie(config, HINT_COOKIE, "", "/", Duration::ZERO)
}

/// The server remembered for `user`, if the request has a valid hint made for them.
fn hinted_guild(config: &DiscordConfig, headers: &HeaderMap, user: u64) -> Option<u64> {
    cookie_values(headers, HINT_COOKIE)
        .find_map(|token| open::<Hint>(&config.signing_key, HINT_PURPOSE, token, SystemTime::now()))
        .filter(|hint| hint.u == user)
        .map(|hint| hint.g)
}

#[cfg(test)]
mod tests;
