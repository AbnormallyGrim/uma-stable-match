//! Opting a club in: checking the signed-in person leads it, finding its server from its
//! description, sending them to add the bot there, and linking the club once Discord confirms the
//! bot went to that same server and they manage it.

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

use crate::auth::{begin, error, safe_next, send_to_discord, Failure, Flow, Target};
use crate::bot::Discord;
use crate::guild_access::{manages, missing_for_bot};
use crate::invites::servers_in_description;
use crate::leaders::{leadership, led_club, managed_club, Leadership, Unchecked};
use crate::oauth::Authorized;
use crate::store::{ClubStore, SettingsUpdate};

/// Path that starts opting a club in. Needs a session whose linked trainer leads the club.
pub const OPT_IN_PATH: &str = "/api/v1/discord/opt-in";

/// Path of a club, with `{club}` the club's ID. `DELETE` unlinks it from its server and needs a
/// session whose linked trainer leads the club.
pub const UNLINK_PATH: &str = "/api/v1/discord/clubs/{club}";

/// The permissions the bot asks for: View Channels, Manage Channels, Manage Roles, Send Messages,
/// Embed Links, Read Message History and Create Instant Invite.
pub const BOT_PERMISSIONS: u64 =
    (1 << 10) | (1 << 4) | (1 << 28) | (1 << 11) | (1 << 14) | (1 << 16) | 1;

/// What opting a club in needs to know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubSummary {
    /// The club's name.
    pub name: String,
    /// The club's in-game description, if it has one.
    pub description: Option<String>,
    /// The leader's trainer ID, if known.
    pub leader: Option<u64>,
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

    /// Where clubs come from.
    pub(crate) fn clubs(&self) -> &ClubLookup {
        &self.clubs
    }
}

/// Serves the opt-in start route, if opt-in is set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord.opt_in().is_none() {
        return Router::new();
    }
    Router::new()
        .route(OPT_IN_PATH, get(start))
        .route(UNLINK_PATH, delete(unlink))
        .with_state(discord.clone())
        .merge(crate::leaders::router(discord.clone()))
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
    let club = params.club.as_deref().unwrap_or_default();
    let (user, club_id, club) = match led_club(&discord, &headers, club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let config = discord.config();
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
    let (_, club_id, _) = match managed_club(&discord, &headers, &club).await {
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

pub(crate) fn store_unavailable(err: &dyn std::fmt::Display) -> Response {
    tracing::warn!(error = %err, "could not reach the Discord club links");
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "couldn't reach the saved links; try again",
    )
}

/// Finishes an opt-in from the callback. Links the club only when the same person who started it
/// approved, the server in Discord's token response is the one the club's invite points to, they
/// manage that server, they still lead the club and the bot kept every permission it asked for.
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
    match manages(discord.rest(), target.guild, target.user).await {
        Ok(true) => {}
        Ok(false) => {
            leave_unless_linked(discord, target.guild).await;
            return Err(Failure::new(
                "not_manager",
                StatusCode::FORBIDDEN,
                "you need Manage Server on the club's Discord server to add the bot",
            ));
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not check who manages a Discord server");
            return Err(Failure::new(
                "unavailable",
                StatusCode::BAD_GATEWAY,
                "couldn't check your rights on the server; try again",
            ));
        }
    }
    match leadership(discord, target.user, target.club).await {
        Ok(Leadership::Leads(_)) => {}
        Ok(_) => {
            leave_unless_linked(discord, target.guild).await;
            return Err(Failure::new(
                "not_leader",
                StatusCode::FORBIDDEN,
                "only the club's leader can add the bot",
            ));
        }
        Err(Unchecked::Store(_) | Unchecked::Lookup) => {
            return Err(Failure::new(
                "unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                "couldn't check the club's leader; try again",
            ));
        }
    }
    match missing_for_bot(
        discord.rest(),
        target.guild,
        discord.config().app_id.get(),
        BOT_PERMISSIONS,
    )
    .await
    {
        Ok(0) => {}
        Ok(_) => {
            leave_unless_linked(discord, target.guild).await;
            return Err(Failure::new(
                "missing_permissions",
                StatusCode::BAD_REQUEST,
                "the bot was added without every permission it needs; start again and leave them all ticked",
            ));
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not check the bot's permissions on a Discord server");
            return Err(Failure::new(
                "unavailable",
                StatusCode::BAD_GATEWAY,
                "couldn't check the bot's permissions on the server; try again",
            ));
        }
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
    // Tickets go to the leader who set the club up.
    let leader = SettingsUpdate {
        leader: Some(target.user),
        ..SettingsUpdate::default()
    };
    opt_in
        .store
        .update(target.club, target.guild, &leader)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "could not note a club's leader");
            Failure::new(
                "unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                "couldn't save the link; try again",
            )
        })?;
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
