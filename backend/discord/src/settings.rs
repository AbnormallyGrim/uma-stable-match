//! A club's Discord settings, for the club's leader.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Deserializer, Serialize};

use crate::auth::error;
use crate::bot::Discord;
use crate::error::Error;
use crate::leaders::managed_club;
use crate::opt_in::store_unavailable;
use crate::store::{ClubRecord, SettingsUpdate};

/// Path of a club's settings, with `{club}` the club's ID. `GET` reads them and `PATCH` changes
/// them. Both need a session whose linked trainer leads the club, checked with uma.moe each time.
pub const SETTINGS_PATH: &str = "/api/v1/discord/clubs/{club}/settings";

/// Discord's channel type for a category.
const CATEGORY: u8 = 4;

/// Discord's error codes for a server the bot can't see: Missing Access and Unknown Guild. Both
/// are what's left after the bot is kicked.
const BOT_GONE: [u32; 2] = [50001, 10004];

pub(crate) fn router(discord: Discord) -> Router {
    Router::new()
        .route(SETTINGS_PATH, get(read).patch(change))
        .with_state(discord)
}

#[derive(Serialize)]
struct Named {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct Settings {
    guild_id: String,
    category: Option<String>,
    auto_join: bool,
    open: bool,
    problem: Option<String>,
    /// The server's categories, to pick the ticket category from.
    categories: Vec<Named>,
}

#[derive(Deserialize)]
struct Channel {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: u8,
}

/// The change to make. Every field is optional; `category: null` clears the category.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    #[serde(default, deserialize_with = "nullable")]
    category: Option<Option<String>>,
    auto_join: Option<bool>,
    open: Option<bool>,
}

/// Tells a missing field from a `null` one.
fn nullable<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

async fn read(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (_, _, record) = match managed_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    match categories(&discord, record.guild_id).await {
        Ok(Some(categories)) => Json(view(&record, categories)).into_response(),
        Ok(None) => forget_server(&discord, record.guild_id).await,
        Err(response) => *response,
    }
}

async fn change(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
    patch: Result<Json<Patch>, JsonRejection>,
) -> Response {
    let (user, club_id, record) = match managed_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Ok(Json(patch)) = patch else {
        return error(StatusCode::BAD_REQUEST, "invalid settings");
    };
    let categories = match categories(&discord, record.guild_id).await {
        Ok(Some(categories)) => categories,
        Ok(None) => return forget_server(&discord, record.guild_id).await,
        Err(response) => return *response,
    };
    let mut update = match checked(&patch, &categories) {
        Ok(update) => update,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    // The leader who changes the settings is the one tickets go to.
    update.leader = Some(user);
    let Some(opt_in) = discord.opt_in() else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    match opt_in
        .store()
        .update(club_id, record.guild_id, &update)
        .await
    {
        Ok(true) => {}
        Ok(false) => {
            return error(
                StatusCode::CONFLICT,
                "the club was moved to another server or unlinked; reload",
            )
        }
        Err(err) => return store_unavailable(&err),
    }
    match opt_in.store().club(club_id).await {
        Ok(Some(record)) => Json(view(&record, categories)).into_response(),
        Ok(None) => error(StatusCode::CONFLICT, "the club was unlinked; reload"),
        Err(err) => store_unavailable(&err),
    }
}

/// The update `patch` asks for, if its category is one of the server's categories.
fn checked(patch: &Patch, categories: &[Named]) -> Result<SettingsUpdate, &'static str> {
    let category = match &patch.category {
        None => None,
        Some(None) => Some(None),
        Some(Some(id)) => {
            let id: u64 = id.parse().map_err(|_| "category must be a category ID")?;
            if !categories.iter().any(|c| c.id == id.to_string()) {
                return Err("category isn't a category of the club's server");
            }
            Some(Some(id))
        }
    };
    let update = SettingsUpdate {
        category,
        auto_join: patch.auto_join,
        open: patch.open,
        leader: None,
    };
    if update == SettingsUpdate::default() {
        return Err("no settings to change");
    }
    Ok(update)
}

/// Unlinks every club on a server the bot was kicked from, so its leaders can add the bot again.
/// Answers as for a club that isn't linked.
async fn forget_server(discord: &Discord, guild_id: u64) -> Response {
    let Some(opt_in) = discord.opt_in() else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    let store = opt_in.store();
    let clubs = match store.clubs_on(guild_id).await {
        Ok(clubs) => clubs,
        Err(err) => return store_unavailable(&err),
    };
    for club in clubs {
        if let Err(err) = store.unlink(club).await {
            return store_unavailable(&err);
        }
        metrics::counter!("discord_unlinks_total", "result" => "bot_removed").increment(1);
    }
    error(StatusCode::NOT_FOUND, "that club isn't linked to a server")
}

/// The server's categories, by name; `None` if the bot is no longer in the server.
async fn categories(discord: &Discord, guild_id: u64) -> Result<Option<Vec<Named>>, Box<Response>> {
    let channels: Vec<Channel> = match discord
        .rest()
        .get(&format!("/guilds/{guild_id}/channels"))
        .await
    {
        Ok(channels) => channels,
        Err(Error::Api {
            code: Some(code), ..
        }) if BOT_GONE.contains(&code) => return Ok(None),
        Err(err) => {
            tracing::warn!(error = %err, "could not list a Discord server's categories");
            return Err(Box::new(error(
                StatusCode::BAD_GATEWAY,
                "couldn't read the server's categories; try again",
            )));
        }
    };
    let mut categories: Vec<Named> = channels
        .into_iter()
        .filter(|channel| channel.kind == CATEGORY)
        .map(|channel| Named {
            id: channel.id,
            name: channel.name,
        })
        .collect();
    categories.sort_by_key(|category| category.name.to_lowercase());
    Ok(Some(categories))
}

fn view(record: &ClubRecord, categories: Vec<Named>) -> Settings {
    Settings {
        guild_id: record.guild_id.to_string(),
        category: record.category.map(|id| id.to_string()),
        auto_join: record.auto_join,
        open: record.open,
        problem: record.problem.clone(),
        categories,
    }
}

#[cfg(test)]
mod tests;
