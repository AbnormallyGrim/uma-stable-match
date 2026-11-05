//! Who leads a club. Only the club's leader in the game can add the bot, change the club's
//! settings or remove the bot: the leader's trainer ID, read fresh from the club, must be the one
//! the signed-in person linked.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::auth::{error, private, session_user};
use crate::bot::Discord;
use crate::opt_in::{store_unavailable, ClubSummary};
use crate::store::ClubRecord;

/// Path that says which trainer the signed-in person linked, the club it's in and whether it
/// leads it. `GET`.
pub const MY_CLUB_PATH: &str = "/api/v1/discord/my-club";

/// Where a person stands with a club.
pub(crate) enum Leadership {
    /// One of their linked trainers leads it.
    Leads(ClubSummary),
    /// They have no trainer linked.
    NoTrainer,
    /// None of their linked trainers leads it.
    NotLeader,
    /// The club doesn't exist.
    NoClub,
}

/// Why leadership couldn't be checked.
pub(crate) enum Unchecked {
    /// The trainer links couldn't be read.
    Store(String),
    /// The club couldn't be looked up.
    Lookup,
}

pub(crate) fn router(discord: Discord) -> Router {
    Router::new()
        .route(MY_CLUB_PATH, get(my_club))
        .with_state(discord)
}

/// Whether `user` leads club `club_id`.
pub(crate) async fn leadership(
    discord: &Discord,
    user: u64,
    club_id: u64,
) -> Result<Leadership, Unchecked> {
    let (Some(opt_in), Some(links)) = (discord.opt_in(), discord.trainer_links()) else {
        return Ok(Leadership::NoClub);
    };
    let linked = links
        .store()
        .linked(user)
        .await
        .map_err(|err| Unchecked::Store(err.to_string()))?;
    if linked.is_empty() {
        return Ok(Leadership::NoTrainer);
    }
    let club = match opt_in.clubs().get(club_id).await {
        Ok(Some(club)) => club,
        Ok(None) => return Ok(Leadership::NoClub),
        Err(_) => return Err(Unchecked::Lookup),
    };
    Ok(match club.leader {
        Some(leader) if linked.contains(&leader) => Leadership::Leads(club),
        _ => Leadership::NotLeader,
    })
}

/// The signed-in user, the club with this ID, as `club`, and what uma.moe says about it, when the
/// user leads it. The `Err` is the response to send.
pub(crate) async fn led_club(
    discord: &Discord,
    headers: &HeaderMap,
    club: &str,
) -> Result<(u64, u64, ClubSummary), Box<Response>> {
    let fail = |status, message| Err(Box::new(error(status, message)));
    if discord.opt_in().is_none() || discord.trainer_links().is_none() {
        return fail(StatusCode::NOT_FOUND, "not found");
    }
    let Some(user) = session_user(discord.config(), headers) else {
        return fail(StatusCode::UNAUTHORIZED, "sign in with Discord first");
    };
    let Ok(club_id) = club.parse::<u64>() else {
        return fail(StatusCode::BAD_REQUEST, "club must be a club ID");
    };
    let found = match leadership(discord, user, club_id).await {
        Ok(found) => found,
        Err(Unchecked::Store(err)) => return Err(Box::new(store_unavailable(&err))),
        Err(Unchecked::Lookup) => {
            return fail(StatusCode::BAD_GATEWAY, "couldn't load the club; try again")
        }
    };
    let forbidden = |message| {
        metrics::counter!("discord_club_access_total", "result" => "forbidden").increment(1);
        fail(StatusCode::FORBIDDEN, message)
    };
    match found {
        Leadership::Leads(summary) => {
            metrics::counter!("discord_club_access_total", "result" => "leader").increment(1);
            Ok((user, club_id, summary))
        }
        Leadership::NoTrainer => forbidden("link your trainer ID in Settings first"),
        Leadership::NotLeader => forbidden("only the club's leader can do this"),
        Leadership::NoClub => fail(StatusCode::NOT_FOUND, "club not found"),
    }
}

/// The club with this ID, as `club`, and its link, when the signed-in user leads it and the bot is
/// in its server. The `Err` is the response to send.
pub(crate) async fn managed_club(
    discord: &Discord,
    headers: &HeaderMap,
    club: &str,
) -> Result<(u64, ClubRecord), Box<Response>> {
    let (_, club_id, _) = led_club(discord, headers, club).await?;
    let Some(opt_in) = discord.opt_in() else {
        return Err(Box::new(error(StatusCode::NOT_FOUND, "not found")));
    };
    match opt_in.store().club(club_id).await {
        Ok(Some(record)) => Ok((club_id, record)),
        Ok(None) => Err(Box::new(error(
            StatusCode::NOT_FOUND,
            "that club isn't linked to a server",
        ))),
        Err(err) => Err(Box::new(store_unavailable(&err))),
    }
}

/// The signed-in person's linked trainer, its club and whether it leads it. A person who linked
/// more than one trainer, before only one was allowed, gets the first one that leads a club.
async fn my_club(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let (Some(opt_in), Some(links)) = (discord.opt_in(), discord.trainer_links()) else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    let Some(user) = session_user(discord.config(), &headers) else {
        return error(StatusCode::UNAUTHORIZED, "sign in with Discord first");
    };
    let linked = match links.store().linked(user).await {
        Ok(linked) => linked,
        Err(err) => return store_unavailable(&err),
    };
    let unavailable = || {
        error(
            StatusCode::BAD_GATEWAY,
            "couldn't load your trainer's club; try again",
        )
    };
    let mut first: Option<Value> = None;
    for trainer in linked {
        let club = match links.trainers().get(trainer).await {
            Ok(Some(profile)) => profile.club,
            Ok(None) => None,
            Err(_) => return unavailable(),
        };
        let club = match club {
            Some(id) => match opt_in.clubs().get(id).await {
                Ok(Some(summary)) => Some(json!({
                    "id": id.to_string(),
                    "name": summary.name,
                    "leader": summary.leader == Some(trainer),
                })),
                Ok(None) => None,
                Err(_) => return unavailable(),
            },
            None => None,
        };
        let leads = club.as_ref().is_some_and(|club| club["leader"] == true);
        let found = json!({ "trainer": trainer.to_string(), "club": club });
        if leads {
            return private(Json(found));
        }
        first.get_or_insert(found);
    }
    private(Json(
        first.unwrap_or_else(|| json!({ "trainer": null, "club": null })),
    ))
}
