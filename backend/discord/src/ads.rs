//! The board of ads: trainers post that they're looking for a club, and leaders of clubs that take
//! applications through Discord find them and invite them to apply.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ad_store::{
    Ad, AdStore, DiscordWish, Invited, Language, NewAd, MAX_RANK_SPAN, RANKS, STORED_FOR,
    VISIBLE_FOR,
};
use crate::apply::linked_trainer;
use crate::auth::{error, private, signed_in};
use crate::bot::Discord;
use crate::calendar::iso;
use crate::leaders::{led_club, managed_club};
use crate::opt_in::store_unavailable;

/// Path of the signed-in trainer's ad. `GET` it (`null` without one), `PUT` it to post or replace
/// it, `DELETE` it.
pub const MY_AD_PATH: &str = "/api/v1/discord/ads/me";

/// Path that renews the signed-in trainer's ad, with fresh fan numbers. `POST`.
pub const MY_AD_RENEW_PATH: &str = "/api/v1/discord/ads/me/renew";

/// Path of a club's invite to the signed-in trainer, with `{club}` the club's ID. `DELETE` it to
/// dismiss it.
pub const MY_AD_INVITE_PATH: &str = "/api/v1/discord/ads/me/invites/{club}";

/// Path that lists one page of the ads on the board, filtered and fastest first. `GET`; needs a
/// session. See the crate's README for the query parameters. With `club`, an ID of a club the
/// signed-in person leads, each ad also says whether that club already invited its trainer.
pub const ADS_PATH: &str = "/api/v1/discord/ads";

/// Path that invites the trainer behind an ad to apply to a club, with `{club}` the club's ID and
/// `{ad}` the ad's. `POST`; needs a session whose linked trainer leads a club that takes
/// applications.
pub const CLUB_AD_INVITE_PATH: &str = "/api/v1/discord/clubs/{club}/ads/{ad}/invite";

/// What a trainer sends to post an ad.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdBody {
    rank_min: u8,
    rank_max: u8,
    discord: String,
    language: String,
}

/// Serves the ad routes, if applying and ads are set up.
pub(crate) fn router(discord: Discord) -> Router {
    if discord
        .applying()
        .and_then(|applying| applying.ads())
        .is_none()
        || discord.opt_in().is_none()
    {
        return Router::new();
    }
    Router::new()
        .route(MY_AD_PATH, get(mine).put(post_ad).delete(remove))
        .route(MY_AD_RENEW_PATH, post(renew))
        .route(MY_AD_INVITE_PATH, delete(dismiss))
        .route(ADS_PATH, get(board))
        .route(CLUB_AD_INVITE_PATH, post(invite))
        .with_state(discord)
}

fn ads(discord: &Discord) -> Result<&AdStore, Box<Response>> {
    discord
        .applying()
        .and_then(|applying| applying.ads())
        .ok_or_else(|| Box::new(error(StatusCode::NOT_FOUND, "not found")))
}

fn bad(message: &'static str) -> Response {
    error(StatusCode::BAD_REQUEST, message)
}

/// What an ad shows: the trainer's numbers and what they're looking for.
fn summary(ad: &Ad) -> Value {
    let details = &ad.details;
    json!({
        "trainerName": details.trainer_name,
        "trainerId": details.trainer.to_string(),
        "currentClub": details.current_club,
        "pace": details.pace,
        "fansThisMonth": details.fans_this_month,
        "rankMin": details.rank_min,
        "rankMax": details.rank_max,
        "discord": details.discord.as_str(),
        "language": details.language.as_str(),
        "postedAt": iso(ad.created_at),
        "visibleUntil": iso(ad.visible_until),
    })
}

/// The trainer's own ad, with the clubs that invited them.
async fn own(discord: &Discord, ad: &Ad) -> Value {
    let mut body = summary(ad);
    let mut invites = Vec::new();
    for invite in &ad.invites {
        let name = match discord.opt_in() {
            Some(opt_in) => opt_in
                .clubs()
                .get(invite.club)
                .await
                .ok()
                .flatten()
                .map(|club| club.name),
            None => None,
        };
        invites.push(json!({
            "clubId": invite.club.to_string(),
            "clubName": name,
            "sentAt": iso(invite.sent_at),
        }));
    }
    body["expiresAt"] = json!(iso(
        ad.visible_until - VISIBLE_FOR.as_secs() + STORED_FOR.as_secs()
    ));
    body["onBoard"] = json!(ad.visible_until > now());
    body["invites"] = json!(invites);
    body
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

async fn mine(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    match ads.get(user).await {
        Ok(Some(ad)) => private(Json(own(&discord, &ad).await)),
        Ok(None) => private(Json(Value::Null)),
        Err(err) => store_unavailable(&err),
    }
}

async fn post_ad(
    State(discord): State<Discord>,
    headers: HeaderMap,
    body: Result<Json<AdBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return bad("the ad isn't valid");
    };
    let (rank_min, rank_max) = (body.rank_min, body.rank_max);
    if !RANKS.contains(&rank_min) || !RANKS.contains(&rank_max) {
        return bad("club ranks go from 1 (D) to 11 (SS)");
    }
    if rank_min > rank_max {
        return bad("the lowest club rank can't be above the highest");
    }
    if rank_max - rank_min >= MAX_RANK_SPAN {
        return bad("an ad can ask for at most three club ranks");
    }
    let (Some(wish), Some(language)) = (
        DiscordWish::parse(&body.discord),
        Language::parse(&body.language),
    ) else {
        return bad("the Discord wish or the language isn't one of the choices");
    };
    let wanted = Wanted {
        rank_min,
        rank_max,
        discord: wish,
        language,
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    save(&discord, user, wanted, "posted").await
}

/// What a trainer asks for in an ad.
#[derive(Clone, Copy)]
struct Wanted {
    rank_min: u8,
    rank_max: u8,
    discord: DiscordWish,
    language: Language,
}

/// Saves the trainer's ad with fresh numbers from their linked trainer, and answers with it.
async fn save(discord: &Discord, user: u64, wanted: Wanted, result: &'static str) -> Response {
    let (Some(applying), Ok(ads)) = (discord.applying(), ads(discord)) else {
        return error(StatusCode::NOT_FOUND, "not found");
    };
    let trainer = match linked_trainer(discord, user).await {
        Ok(trainer) => trainer,
        Err(failure) => return failure.respond(&HeaderMap::new()),
    };
    let details = match applying.applicant(trainer).await {
        Ok(Some(details)) => details,
        Ok(None) => return error(StatusCode::NOT_FOUND, "your trainer wasn't found"),
        Err(_) => {
            return error(
                StatusCode::BAD_GATEWAY,
                "couldn't load your trainer; try again",
            )
        }
    };
    let new = NewAd {
        user,
        trainer,
        trainer_name: details.name,
        current_club: details.club_name,
        pace: details.pace_per_day,
        fans_this_month: details.fans_this_month,
        rank_min: wanted.rank_min,
        rank_max: wanted.rank_max,
        discord: wanted.discord,
        language: wanted.language,
    };
    match ads.save(&new).await {
        Ok(ad) => {
            metrics::counter!("discord_ads_total", "result" => result).increment(1);
            private(Json(own(discord, &ad).await))
        }
        Err(err) => store_unavailable(&err),
    }
}

async fn renew(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let ad = match ads.get(user).await {
        Ok(Some(ad)) => ad,
        Ok(None) => return error(StatusCode::NOT_FOUND, "you have no ad to renew"),
        Err(err) => return store_unavailable(&err),
    };
    let details = &ad.details;
    let wanted = Wanted {
        rank_min: details.rank_min,
        rank_max: details.rank_max,
        discord: details.discord,
        language: details.language,
    };
    save(&discord, user, wanted, "renewed").await
}

async fn remove(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    match ads.delete(user).await {
        Ok(_) => {
            metrics::counter!("discord_ads_total", "result" => "deleted").increment(1);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => store_unavailable(&err),
    }
}

async fn dismiss(
    State(discord): State<Discord>,
    Path(club): Path<String>,
    headers: HeaderMap,
) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let Ok(club) = club.parse::<u64>() else {
        return bad("club must be a club ID");
    };
    match ads.dismiss(user, club).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error(StatusCode::NOT_FOUND, "that club hasn't invited you"),
        Err(err) => store_unavailable(&err),
    }
}

/// Ads on one page of the board.
pub const PAGE_SIZE: usize = 20;

#[derive(Deserialize)]
struct BoardParams {
    club: Option<String>,
    page: Option<String>,
    rank_min: Option<String>,
    rank_max: Option<String>,
    has_discord: Option<String>,
    language: Option<String>,
}

/// How a leader narrows the board down, by what their club is like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoardFilters {
    /// The ad's lowest rank must be at or above this; `None` leaves it unfiltered.
    pub(crate) rank_min: Option<u8>,
    /// The ad's highest rank must be at or below this; `None` leaves it unfiltered.
    pub(crate) rank_max: Option<u8>,
    /// Whether the club has a Discord server: without one, trainers who require one are left out.
    /// `None` leaves it unfiltered.
    pub(crate) has_discord: Option<bool>,
    /// Ads for another language are left out; `Any` leaves it unfiltered.
    pub(crate) language: Language,
}

impl BoardFilters {
    /// The filters and page in `params`, or why they can't be read. A filter left out is unset, and
    /// the page is the first.
    fn parse(params: &BoardParams) -> Result<(Self, usize), &'static str> {
        let rank = |text: &Option<String>| match text {
            None => Ok(None),
            Some(text) => text
                .parse::<u8>()
                .ok()
                .filter(|rank| RANKS.contains(rank))
                .map(Some)
                .ok_or("club ranks go from 1 (D) to 11 (SS)"),
        };
        let (mut rank_min, mut rank_max) = (rank(&params.rank_min)?, rank(&params.rank_max)?);
        if let (Some(low), Some(high)) = (rank_min, rank_max) {
            // Given the wrong way round, they're swapped.
            (rank_min, rank_max) = (Some(low.min(high)), Some(low.max(high)));
        }
        let has_discord = match params.has_discord.as_deref() {
            None => None,
            Some("true") => Some(true),
            Some("false") => Some(false),
            Some(_) => return Err("has_discord must be true or false"),
        };
        let language = match params.language.as_deref() {
            None => Language::Any,
            Some(text) => Language::parse(text).ok_or("the language isn't one of the choices")?,
        };
        let page = match params.page.as_deref() {
            None => 1,
            Some(text) => text
                .parse::<usize>()
                .ok()
                .filter(|page| *page >= 1)
                .ok_or("page must be a number from 1")?,
        };
        Ok((
            Self {
                rank_min,
                rank_max,
                has_discord,
                language,
            },
            page,
        ))
    }

    /// Whether the ad passes every filter that's set: its lowest rank is at or above `rank_min`,
    /// its highest at or below `rank_max`, it doesn't require a Discord server the club lacks, and
    /// its language is the club's or any.
    pub(crate) fn matches(&self, ad: &Ad) -> bool {
        let details = &ad.details;
        self.rank_min.is_none_or(|low| details.rank_min >= low)
            && self.rank_max.is_none_or(|high| details.rank_max <= high)
            && (self.has_discord != Some(false) || details.discord != DiscordWish::Required)
            && (self.language == Language::Any
                || details.language == Language::Any
                || details.language == self.language)
    }
}

/// One page of the ads that match, fastest first and those without a pace last, with the page
/// shown (kept within range), how many pages there are (at least one) and how many ads match.
pub(crate) fn board_page<'a>(
    ads: &'a [Ad],
    filters: &BoardFilters,
    page: usize,
) -> (Vec<&'a Ad>, usize, usize, usize) {
    let mut matching: Vec<&Ad> = ads.iter().filter(|ad| filters.matches(ad)).collect();
    matching.sort_by(|a, b| {
        let pace = |ad: &Ad| ad.details.pace.unwrap_or(-1.0);
        pace(b).total_cmp(&pace(a))
    });
    let total = matching.len();
    let pages = total.div_ceil(PAGE_SIZE).max(1);
    let page = page.clamp(1, pages);
    let shown = matching
        .into_iter()
        .skip((page - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .collect();
    (shown, page, pages, total)
}

async fn board(
    State(discord): State<Discord>,
    Query(params): Query<BoardParams>,
    headers: HeaderMap,
) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    let (filters, page) = match BoardFilters::parse(&params) {
        Ok(found) => found,
        Err(message) => return bad(message),
    };
    // The ads are for anyone signed in; only a club's leader can ask what the club invited.
    let club = match params.club {
        Some(club) => match led_club(&discord, &headers, &club).await {
            Ok((_, club_id, _)) => Some(club_id),
            Err(response) => return *response,
        },
        None => match signed_in(&discord, &headers).await {
            Ok(_) => None,
            Err(response) => return *response,
        },
    };
    let listed = match ads.board().await {
        Ok(listed) => listed,
        Err(err) => return store_unavailable(&err),
    };
    let (shown, page, pages, total) = board_page(&listed, &filters, page);
    let out: Vec<Value> = shown
        .iter()
        .map(|ad| {
            let mut item = summary(ad);
            item["id"] = json!(ad.id);
            item["invited"] =
                json!(club.is_some_and(|club| ad.invites.iter().any(|invite| invite.club == club)));
            item
        })
        .collect();
    private(Json(json!({
        "ads": out,
        "page": page,
        "pages": pages,
        "total": total,
    })))
}

async fn invite(
    State(discord): State<Discord>,
    Path((club, ad)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let ads = match ads(&discord) {
        Ok(ads) => ads,
        Err(response) => return *response,
    };
    if ad.len() != 32 || !ad.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return bad("the ad isn't valid");
    }
    let (_, club_id, record) = match managed_club(&discord, &headers, &club).await {
        Ok(found) => found,
        Err(response) => return *response,
    };
    if !record.open || record.problem.is_some() {
        return error(
            StatusCode::CONFLICT,
            "your club doesn't take applications through Discord yet",
        );
    }
    match ads.invite(&ad, club_id).await {
        Ok(Invited::Sent | Invited::Already) => {
            metrics::counter!("discord_ad_invites_total", "result" => "sent").increment(1);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Invited::Gone) => error(StatusCode::NOT_FOUND, "that ad is no longer on the board"),
        Ok(Invited::Full) => {
            metrics::counter!("discord_ad_invites_total", "result" => "full").increment(1);
            error(StatusCode::CONFLICT, "that trainer has too many invites")
        }
        Err(err) => store_unavailable(&err),
    }
}

#[cfg(test)]
mod tests;
