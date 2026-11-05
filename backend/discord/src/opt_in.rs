//! Opting a club in: finding its server from its description, sending its leader to add the bot
//! there, and linking the club once Discord confirms the bot went to that same server.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::Router;
use serde::Deserialize;

use crate::auth::{begin, error, safe_next, send_to_discord, session_user, Failure, Flow, Target};
use crate::bot::Discord;
use crate::guild_access::{rights, Access, Rights};
use crate::invites::servers_in_description;
use crate::oauth::Authorized;
use crate::store::{ClubRecord, ClubStore};

/// Path that starts opting a club in. Needs a session.
pub const OPT_IN_PATH: &str = "/api/v1/discord/opt-in";

/// Path of a club, with `{club}` the club's ID. `DELETE` unlinks it from its server and needs a
/// session with Manage Server on the club's server; `GET` says what access the session has.
pub const UNLINK_PATH: &str = "/api/v1/discord/clubs/{club}";

/// The permissions the bot asks for: View Channels, Manage Channels, Manage Roles, Send Messages,
/// Embed Links, Read Message History and Create Instant Invite.
pub const BOT_PERMISSIONS: u64 =
    (1 << 10) | (1 << 4) | (1 << 28) | (1 << 11) | (1 << 14) | (1 << 16) | 1;

/// What opting a club in needs to know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubSummary {
    /// The club's in-game description, if it has one.
    pub description: Option<String>,
}

/// The club lookup failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the club could not be looked up")]
pub struct LookupError;

type Lookup = dyn Fn(u64) -> Pin<Box<dyn Future<Output = Result<Option<ClubSummary>, LookupError>> + Send>>
    + Send
    + Sync;

/// Looks clubs up by ID. Cheap to clone.
#[derive(Clone)]
pub struct ClubLookup(Arc<Lookup>);

impl ClubLookup {
    /// A lookup that calls `lookup`, which answers `Ok(None)` for a club that doesn't exist.
    pub fn new<F, Fut>(lookup: F) -> Self
    where
        F: Fn(u64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<ClubSummary>, LookupError>> + Send + 'static,
    {
        Self(Arc::new(move |id| Box::pin(lookup(id))))
    }

    /// The club with this ID: `None` if it doesn't exist.
    pub async fn get(&self, club_id: u64) -> Result<Option<ClubSummary>, LookupError> {
        (self.0)(club_id).await
    }
}

impl std::fmt::Debug for ClubLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClubLookup").finish_non_exhaustive()
    }
}

/// What opting clubs in uses: where links are kept and where clubs come from.
#[derive(Debug, Clone)]
pub struct OptIn {
    store: ClubStore,
    clubs: ClubLookup,
}

impl OptIn {
    /// Opt-in that keeps links in `store` and finds clubs with `clubs`.
    pub fn new(store: ClubStore, clubs: ClubLookup) -> Self {
        Self { store, clubs }
    }

    /// Where links are kept.
    pub fn store(&self) -> &ClubStore {
        &self.store
    }
}

/// Serves the opt-in start route, if opt-in is set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord.opt_in().is_none() {
        return Router::new();
    }
    Router::new()
        .route(OPT_IN_PATH, get(start))
        .route(UNLINK_PATH, delete(unlink).get(crate::clubs::access))
        .with_state(discord.clone())
        .merge(crate::clubs::router(discord.clone()))
        .merge(crate::settings::router(discord))
}

#[derive(Deserialize)]
struct StartParams {
    club: Option<String>,
    next: Option<String>,
}

async fn start(
    State(discord): State<Discord>,
    Query(params): Query<StartParams>,
    headers: HeaderMap,
) -> Response {
    let (Some(opt_in), config) = (discord.opt_in(), discord.config()) else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    let Some(user) = session_user(config, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "sign in with Discord first");
    };
    let Some(club_id) = params.club.as_deref().and_then(|id| id.parse::<u64>().ok()) else {
        return error(StatusCode::BAD_REQUEST, "club must be a club ID");
    };
    let club = match opt_in.clubs.get(club_id).await {
        Ok(Some(club)) => club,
        Ok(None) => return error(StatusCode::NOT_FOUND, "club not found"),
        Err(_) => return error(StatusCode::BAD_GATEWAY, "couldn't load the club; try again"),
    };
    let description = club.description.as_deref().unwrap_or_default();
    let servers = match servers_in_description(discord.rest(), description).await {
        Ok(servers) => servers,
        Err(err) => {
            tracing::warn!(error = %err, "could not resolve a club's Discord invites");
            return error(
                StatusCode::BAD_GATEWAY,
                "couldn't check the club's Discord invite; try again",
            );
        }
    };
    let guild = match servers.as_slice() {
        [guild] => *guild,
        [] => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "the club's description has no working Discord invite",
            )
        }
        _ => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "the club's description has invites to more than one Discord server",
            )
        }
    };

    let next = safe_next(params.next.as_deref().unwrap_or("/"));
    let target = Target {
        club: club_id,
        guild,
        user,
    };
    let (state, nonce) = begin(config, Flow::OptIn, next, Some(target), SystemTime::now());
    send_to_discord(
        config,
        Flow::OptIn,
        &state,
        &nonce,
        &[
            ("guild_id", &guild.to_string()),
            ("disable_guild_select", "true"),
            ("permissions", &BOT_PERMISSIONS.to_string()),
        ],
    )
}

async fn unlink(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (club_id, _, _) = match managed_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(opt_in) = discord.opt_in() else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    match opt_in.store.unlink(club_id).await {
        Ok(left) => {
            // Nothing needs the bot on a server once its last club is gone.
            if let Some(left) = left.filter(|left| left.remaining == 0) {
                leave(&discord, left.guild_id).await;
            }
            metrics::counter!("discord_unlinks_total", "result" => "success").increment(1);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => store_unavailable(&err),
    }
}

/// The club with this ID, as `club`, and the manager's access. The signed-in user must own its
/// server or have Administrator or Manage Server there, the rights needed to add the bot, checked
/// with Discord each time. The `Err` is the response to send.
pub(crate) async fn managed_club(
    discord: &Discord,
    headers: &HeaderMap,
    club: &str,
) -> Result<(u64, ClubRecord, Access), Box<Response>> {
    let (_, club_id, record, rights) = member_club(discord, headers, club).await?;
    if !rights.manage {
        metrics::counter!("discord_club_access_total", "result" => "forbidden").increment(1);
        return Err(Box::new(error(
            StatusCode::FORBIDDEN,
            "you need Manage Server on the club's Discord server for this",
        )));
    }
    Ok((
        club_id,
        record,
        Access {
            roles: rights.roles,
        },
    ))
}

/// The signed-in user, the club with this ID, as `club`, and what the user can do in the club's
/// server, checked with Discord each time. The caller decides what the rights allow for this club.
/// The `Err` is the response to send.
pub(crate) async fn member_club(
    discord: &Discord,
    headers: &HeaderMap,
    club: &str,
) -> Result<(u64, u64, ClubRecord, Rights), Box<Response>> {
    let (Some(opt_in), config) = (discord.opt_in(), discord.config()) else {
        return Err(Box::new(error(StatusCode::NOT_FOUND, "not found")));
    };
    let Some(user) = session_user(config, headers) else {
        return Err(Box::new(error(
            StatusCode::UNAUTHORIZED,
            "sign in with Discord first",
        )));
    };
    let Ok(club_id) = club.parse::<u64>() else {
        return Err(Box::new(error(
            StatusCode::BAD_REQUEST,
            "club must be a club ID",
        )));
    };
    let record = match opt_in.store.club(club_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Err(Box::new(error(
                StatusCode::NOT_FOUND,
                "that club isn't linked to a server",
            )))
        }
        Err(err) => return Err(Box::new(store_unavailable(&err))),
    };
    let rights = match rights(discord.rest(), record.guild_id, user).await {
        Ok(rights) => rights,
        Err(err)
            if matches!(
                err.status(),
                Some(StatusCode::FORBIDDEN | StatusCode::NOT_FOUND)
            ) =>
        {
            None
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not check who manages a Discord server");
            return Err(Box::new(error(
                StatusCode::BAD_GATEWAY,
                "couldn't check your rights on the server; try again",
            )));
        }
    };
    let Some(rights) = rights else {
        metrics::counter!("discord_club_access_total", "result" => "forbidden").increment(1);
        return Err(Box::new(error(
            StatusCode::FORBIDDEN,
            "you need Manage Server or a leader role on the club's Discord server for this",
        )));
    };
    Ok((user, club_id, record, rights))
}

pub(crate) fn store_unavailable(err: &dyn std::fmt::Display) -> Response {
    tracing::warn!(error = %err, "could not reach the Discord club links");
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "couldn't reach the saved links; try again",
    )
}

/// Finishes an opt-in from the callback. Links the club only when the same person who started it
/// approved, and the server in Discord's token response is the one the club's invite points to.
pub(crate) async fn install(
    discord: &Discord,
    target: Target,
    authorized: &Authorized,
) -> Result<(), Failure> {
    let Some(opt_in) = discord.opt_in() else {
        return Err(Failure::new(
            "unavailable",
            StatusCode::NOT_FOUND,
            "not found",
        ));
    };
    if authorized.user_id != target.user {
        return Err(Failure::new(
            "invalid_state",
            StatusCode::BAD_REQUEST,
            "this sign-in link is invalid or has expired; start again",
        ));
    }
    let Some(added_to) = authorized.guild_id else {
        return Err(Failure::new(
            "no_server",
            StatusCode::BAD_REQUEST,
            "the bot wasn't added to a server; start again",
        ));
    };
    if added_to != target.guild {
        leave_unless_linked(discord, added_to).await;
        return Err(Failure::new(
            "wrong_server",
            StatusCode::FORBIDDEN,
            "the bot was added to a different server than the one in the club's description",
        ));
    }
    let moved = opt_in
        .store
        .link(target.club, target.guild)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "could not link a club to its Discord server");
            Failure::new(
                "unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                "couldn't save the link; try again",
            )
        })?;
    // A club that moved may have left a server no club needs the bot on.
    if let Some(left) = moved.filter(|left| left.remaining == 0) {
        leave(discord, left.guild_id).await;
    }
    Ok(())
}

/// Leaves a server the bot shouldn't be in, unless some club is linked to it.
async fn leave_unless_linked(discord: &Discord, guild_id: u64) {
    let Some(opt_in) = discord.opt_in() else {
        return;
    };
    match opt_in.store.clubs_on(guild_id).await {
        Ok(clubs) if clubs.is_empty() => leave(discord, guild_id).await,
        Ok(_) => {}
        // Stays in the server when its clubs can't be read.
        Err(err) => tracing::warn!(error = %err, "could not check a server before leaving it"),
    }
}

async fn leave(discord: &Discord, guild_id: u64) {
    let left = discord
        .rest()
        .delete::<()>(&format!("/users/@me/guilds/{guild_id}"))
        .await;
    if let Err(err) = left {
        tracing::warn!(error = %err, "the bot could not leave a server");
    }
}
