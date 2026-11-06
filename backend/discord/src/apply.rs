//! Applying to a club: the player signs in with permission to join the club's server, and the
//! callback joins them, opens a ticket for the club's leader and saves the application.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::application_store::{Application, ApplicationStore, NewApplication, Sent, Status};
use crate::auth::{
    begin, private, send_to_discord, session_user, signed_in, Failure, Flow, Target,
};
use crate::bot::Discord;
use crate::calendar::iso;
use crate::leaders::managed_club;
use crate::oauth::Authorized;
use crate::opt_in::{store_unavailable, LookupError};
use crate::tickets;

/// Path that starts applying to a club, with `club` the club's ID. `GET`. Sends the browser to
/// Discord, signing the player in first if they aren't.
pub const APPLY_PATH: &str = "/api/v1/discord/apply";

/// Path that lists the signed-in player's applications, newest first. `GET`.
pub const APPLICATIONS_PATH: &str = "/api/v1/discord/applications";

/// Path that lists a club's applications, newest first, with `{club}` the club's ID. `GET`; needs
/// a session whose linked trainer leads the club.
pub const CLUB_APPLICATIONS_PATH: &str = "/api/v1/discord/clubs/{club}/applications";

/// Where a player lands after applying.
const TRAINER_PAGE: &str = "/applications/mine";

/// Most applications a club's list shows.
const MAX_LISTED: usize = 100;

/// What an application shows about the player's trainer.
#[derive(Debug, Clone, PartialEq)]
pub struct Applicant {
    /// The trainer's name.
    pub name: String,
    /// The name of the club the trainer is in, if any.
    pub club_name: Option<String>,
    /// Fans per day, if known.
    pub pace_per_day: Option<f64>,
    /// Fans gained this month, if known.
    pub fans_this_month: Option<i64>,
    /// All-time fan totals and averages, if known.
    pub alltime: Option<Alltime>,
}

/// A trainer's all-time fans.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Alltime {
    /// Total fans.
    pub total_fans: i64,
    /// Days with data.
    pub active_days: i64,
    /// Average fans gained per day.
    pub avg_day: Option<f64>,
    /// Average fans gained per week.
    pub avg_week: Option<f64>,
    /// Average fans gained per month.
    pub avg_month: Option<f64>,
    /// Rank by total fans.
    pub rank: Option<i64>,
}

type Lookup = dyn Fn(u64) -> Pin<Box<dyn Future<Output = Result<Option<Applicant>, LookupError>> + Send>>
    + Send
    + Sync;

/// Looks up what an application shows about a trainer, fresh each time. Cheap to clone.
#[derive(Clone)]
pub struct ApplicantLookup(Arc<Lookup>);

impl ApplicantLookup {
    /// A lookup that calls `lookup`, which answers `Ok(None)` for a trainer that doesn't exist.
    pub fn new<F, Fut>(lookup: F) -> Self
    where
        F: Fn(u64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<Applicant>, LookupError>> + Send + 'static,
    {
        Self(Arc::new(move |id| Box::pin(lookup(id))))
    }

    async fn get(&self, trainer_id: u64) -> Result<Option<Applicant>, LookupError> {
        (self.0)(trainer_id).await
    }
}

impl std::fmt::Debug for ApplicantLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicantLookup").finish_non_exhaustive()
    }
}

/// What taking applications uses: where they're kept and where trainers' details come from.
#[derive(Debug, Clone)]
pub struct Applying {
    store: ApplicationStore,
    applicants: ApplicantLookup,
}

impl Applying {
    /// Applications kept in `store`, with trainers' details from `applicants`.
    pub fn new(store: ApplicationStore, applicants: ApplicantLookup) -> Self {
        Self { store, applicants }
    }
}

/// Serves the apply route and the application lists, if applying is set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord.applying().is_none() || discord.opt_in().is_none() {
        return Router::new();
    }
    Router::new()
        .route(APPLY_PATH, get(start))
        .route(APPLICATIONS_PATH, get(mine))
        .route(CLUB_APPLICATIONS_PATH, get(for_club))
        .with_state(discord)
}

#[derive(Deserialize)]
struct StartParams {
    club: Option<String>,
}

fn failure(label: &'static str, status: StatusCode, message: &'static str) -> Failure {
    Failure::new(label, status, message)
}

fn unavailable() -> Failure {
    failure(
        "unavailable",
        StatusCode::SERVICE_UNAVAILABLE,
        "couldn't reach the saved links; try again",
    )
}

/// The first trainer the user linked, which is the only one since a person can link one.
async fn linked_trainer(discord: &Discord, user: u64) -> Result<u64, Failure> {
    let Some(links) = discord.trainer_links() else {
        return Err(failure("unavailable", StatusCode::NOT_FOUND, "not found"));
    };
    let linked = links.store().linked(user).await.map_err(|err| {
        tracing::warn!(error = %err, "could not read a player's linked trainer");
        unavailable()
    })?;
    linked.first().copied().ok_or_else(|| {
        failure(
            "no_trainer",
            StatusCode::FORBIDDEN,
            "link your trainer ID in Settings first",
        )
    })
}

/// The club's link, if it takes applications.
async fn open_club(discord: &Discord, club: u64) -> Result<crate::store::ClubRecord, Failure> {
    let Some(opt_in) = discord.opt_in() else {
        return Err(failure("unavailable", StatusCode::NOT_FOUND, "not found"));
    };
    let record = opt_in.store().club(club).await.map_err(|err| {
        tracing::warn!(error = %err, "could not read a club's link");
        unavailable()
    })?;
    match record {
        Some(record) if record.open && record.problem.is_none() => Ok(record),
        _ => Err(failure(
            "not_open",
            StatusCode::NOT_FOUND,
            "this club doesn't take applications through Discord",
        )),
    }
}

async fn start(
    State(discord): State<Discord>,
    Query(params): Query<StartParams>,
    headers: HeaderMap,
) -> Response {
    let config = discord.config();
    let club = params.club.as_deref().unwrap_or_default();
    let Ok(club_id) = club.parse::<u64>() else {
        return failure(
            "bad_club",
            StatusCode::BAD_REQUEST,
            "club must be a club ID",
        )
        .respond(&headers);
    };
    let user = match session_user(&discord, &headers).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Sign in, then come back here.
            let next = format!("{APPLY_PATH}?club={club_id}");
            let (state, nonce) = begin(config, Flow::SignIn, &next, None, SystemTime::now());
            return send_to_discord(config, Flow::SignIn, &state, &nonce, &[("prompt", "none")]);
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not check a session");
            return unavailable().respond(&headers);
        }
    };
    let checked = async {
        let record = open_club(&discord, club_id).await?;
        linked_trainer(&discord, user).await?;
        Ok::<_, Failure>(record)
    }
    .await;
    let record = match checked {
        Ok(record) => record,
        Err(failure) => return failure.respond(&headers),
    };
    let target = Target {
        club: club_id,
        guild: record.guild_id,
        user,
    };
    let (state, nonce) = begin(
        config,
        Flow::Apply,
        TRAINER_PAGE,
        Some(target),
        SystemTime::now(),
    );
    send_to_discord(config, Flow::Apply, &state, &nonce, &[])
}

/// Finishes an application from the callback: joins the player to the club's server, opens the
/// ticket and saves the application. Only the person who started it can finish it, and nothing is
/// kept if the ticket can't be opened.
pub(crate) async fn submit(
    discord: &Discord,
    target: Target,
    authorized: &Authorized,
) -> Result<(), Failure> {
    let result = apply(discord, target, authorized).await;
    let label = result.as_ref().err().map_or("success", |err| err.label);
    metrics::counter!("discord_applications_total", "result" => label).increment(1);
    result
}

async fn apply(discord: &Discord, target: Target, authorized: &Authorized) -> Result<(), Failure> {
    let (Some(applying), Some(opt_in)) = (discord.applying(), discord.opt_in()) else {
        return Err(failure("unavailable", StatusCode::NOT_FOUND, "not found"));
    };
    if authorized.user_id != target.user {
        return Err(failure(
            "invalid_state",
            StatusCode::BAD_REQUEST,
            "this sign-in link is invalid or has expired; start again",
        ));
    }
    let record = open_club(discord, target.club).await?;
    if record.guild_id != target.guild {
        return Err(failure(
            "not_open",
            StatusCode::CONFLICT,
            "the club moved to another server; start again",
        ));
    }
    let trainer = linked_trainer(discord, target.user).await?;
    let club = match opt_in.clubs().get(target.club).await {
        Ok(Some(club)) => club,
        Ok(None) => return Err(failure("not_open", StatusCode::NOT_FOUND, "club not found")),
        Err(_) => {
            return Err(failure(
                "unavailable",
                StatusCode::BAD_GATEWAY,
                "couldn't load the club; try again",
            ))
        }
    };
    let details = match applying.applicants.get(trainer).await {
        Ok(Some(details)) => details,
        Ok(None) => {
            return Err(failure(
                "no_trainer",
                StatusCode::NOT_FOUND,
                "your trainer wasn't found",
            ))
        }
        Err(_) => {
            return Err(failure(
                "unavailable",
                StatusCode::BAD_GATEWAY,
                "couldn't load your trainer; try again",
            ))
        }
    };
    let new = NewApplication {
        club: target.club,
        club_name: club.name,
        guild: record.guild_id,
        applicant: target.user,
        trainer,
        trainer_name: details.name.clone(),
        current_club: details.club_name.clone(),
        pace: details.pace_per_day,
        fans_this_month: details.fans_this_month,
    };
    let id = match applying.store.create(&new).await {
        Ok(Sent::Saved(id)) => id,
        Ok(Sent::Pending) => {
            return Err(failure(
                "already_applied",
                StatusCode::CONFLICT,
                "you already have an application to this club waiting for an answer",
            ))
        }
        Ok(Sent::OverLimit) => {
            return Err(failure(
                "too_many_applications",
                StatusCode::TOO_MANY_REQUESTS,
                "you've sent the most applications you can in a day; try again tomorrow",
            ))
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not save an application");
            return Err(unavailable());
        }
    };
    let application = match applying.store.get(&id).await {
        Ok(Some(application)) => application,
        _ => return Err(unavailable()),
    };
    match open_ticket(discord, &record, &application, &details, authorized).await {
        Ok(()) => Ok(()),
        Err(err) => {
            // Nothing is kept for an application that has no ticket.
            if let Err(err) = applying.store.discard(&application).await {
                tracing::warn!(error = %err, "could not take back an application");
            }
            Err(err)
        }
    }
}

/// Joins the applicant to the server, opens their ticket and notes it on the application.
async fn open_ticket(
    discord: &Discord,
    record: &crate::store::ClubRecord,
    application: &Application,
    details: &Applicant,
    authorized: &Authorized,
) -> Result<(), Failure> {
    let guild = record.guild_id;
    let user = application.details.applicant;
    // Applicants are always added to the server, so the ticket can include them.
    let joined: Result<Value, _> = discord
        .rest()
        .put(
            &format!("/guilds/{guild}/members/{user}"),
            &json!({ "access_token": authorized.access_token }),
        )
        .await;
    if let Err(err) = joined {
        tracing::warn!(error = %err, "could not add an applicant to a Discord server");
        return Err(failure(
            "join_failed",
            StatusCode::BAD_GATEWAY,
            "Discord wouldn't add you to the club's server",
        ));
    }
    let (channel, message) = tickets::open(discord, record, application, details)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "could not open an application's ticket");
            failure(
                "ticket_failed",
                StatusCode::BAD_GATEWAY,
                "couldn't open your ticket on the club's server; try again",
            )
        })?;
    let applying = discord.applying().ok_or_else(unavailable)?;
    applying
        .store
        .attach(&application.id, channel, message)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "could not note an application's ticket");
            unavailable()
        })?;
    Ok(())
}

/// The link to a ticket channel.
fn ticket_url(application: &Application) -> Option<String> {
    let channel = application.channel?;
    Some(format!(
        "https://discord.com/channels/{}/{channel}",
        application.details.guild
    ))
}

async fn mine(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let Some(applying) = discord.applying() else {
        return crate::auth::error(StatusCode::NOT_FOUND, "not found");
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let applications = match applying.store.for_applicant(user).await {
        Ok(applications) => applications,
        Err(err) => return store_unavailable(&err),
    };
    let listed: Vec<Value> = applications
        .iter()
        .map(|application| {
            json!({
                "id": application.id,
                "clubId": application.details.club.to_string(),
                "clubName": application.details.club_name,
                "trainerId": application.details.trainer.to_string(),
                "status": application.status.as_str(),
                "submittedAt": iso(application.submitted_at),
                "ticketUrl": ticket_url(application),
            })
        })
        .collect();
    private(Json(listed))
}

async fn for_club(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (_, club_id, _) = match managed_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    let Some(applying) = discord.applying() else {
        return crate::auth::error(StatusCode::NOT_FOUND, "not found");
    };
    let applications = match applying.store.for_club(club_id).await {
        Ok(applications) => applications,
        Err(err) => return store_unavailable(&err),
    };
    let mut listed = Vec::new();
    for application in applications.iter().take(MAX_LISTED) {
        let details = &application.details;
        let name = discord
            .names()
            .get(discord.rest(), details.applicant)
            .await
            .unwrap_or_else(|| details.applicant.to_string());
        listed.push(json!({
            "id": application.id,
            "trainerName": details.trainer_name,
            "trainerId": details.trainer.to_string(),
            "discordName": name,
            "pace": details.pace.unwrap_or(0.0),
            "fansThisMonth": details.fans_this_month.unwrap_or(0),
            "currentClub": details.current_club,
            "status": application.status.as_str(),
            "submittedAt": iso(application.submitted_at),
            "ticketUrl": ticket_url(application),
        }));
    }
    private(Json(listed))
}

/// What a click on a ticket's Accept or Decline button does, as the interaction response. Only
/// the club's leader can answer; a club with no leader noted lets whoever manages the server.
pub(crate) async fn answer(
    discord: &Discord,
    custom_id: &str,
    clicker: Option<u64>,
    manages_server: bool,
) -> Value {
    let reply = |text: &str| json!({ "type": 4, "data": { "content": text, "flags": 1 << 6 } });
    let Some((status, id)) = parse_button(custom_id) else {
        return reply("This action isn't available.");
    };
    let (Some(applying), Some(opt_in)) = (discord.applying(), discord.opt_in()) else {
        return reply("Applications aren't set up.");
    };
    let application = match applying.store.get(id).await {
        Ok(Some(application)) => application,
        Ok(None) => return reply("This application has expired."),
        Err(_) => return reply("Couldn't reach the saved applications; try again."),
    };
    let record = match opt_in.store().club(application.details.club).await {
        Ok(Some(record)) => record,
        Ok(None) => return reply("This club isn't linked to the server any more."),
        Err(_) => return reply("Couldn't reach the saved links; try again."),
    };
    let allowed = match (record.leader, clicker) {
        (Some(leader), Some(clicker)) => leader == clicker,
        (None, _) => manages_server,
        _ => false,
    };
    if !allowed {
        return reply("Only the club's leader can answer applications.");
    }
    match applying.store.answer(id, status).await {
        Ok(true) => {
            metrics::counter!("discord_application_answers_total", "status" => status.as_str())
                .increment(1);
            let who = clicker.map_or_else(String::new, |id| format!(" by <@{id}>"));
            let word = if status == Status::Accepted {
                "✅ Accepted"
            } else {
                "❌ Declined"
            };
            json!({
                "type": 7,
                "data": {
                    "content": format!("{word}{who}"),
                    "components": [],
                    "allowed_mentions": { "parse": [] },
                },
            })
        }
        Ok(false) => reply("This application was already answered."),
        Err(_) => reply("Couldn't save the answer; try again."),
    }
}

/// The status a button's custom ID sets and the application it's for.
fn parse_button(custom_id: &str) -> Option<(Status, &str)> {
    let rest = custom_id.strip_prefix("apply:")?;
    let (action, id) = rest.split_once(':')?;
    let status = match action {
        "accept" => Status::Accepted,
        "decline" => Status::Declined,
        _ => return None,
    };
    (!id.is_empty()).then_some((status, id))
}

#[cfg(test)]
mod tests;
