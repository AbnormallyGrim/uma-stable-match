//! `GET /api/v1/recommendations`: scores the cached clubs for a pace or a trainer.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, RawQuery, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use uma_stable_match_scoring::{
    score_club, score_clubs, Balance, Club, JoinStyle, ScoredClub, ScoringOptions,
    DEFAULT_BALANCE_WEIGHT,
};
use uma_stable_match_umamoe_client::Priority;

use crate::api_error::ApiError;
use crate::app::timestamp;
use crate::app::AppState;
use crate::details::ClubDetails;
use crate::limits::Visitor;
use crate::pace::PaceError;
use crate::pool::{ShortlistFilter, DEFAULT_SHORTLIST_SIZE, TARGET_MEDIAN_RATIO};

const CLUB_URL: &str = "https://uma.moe/circles/";
const DEFAULT_LIMIT: u32 = 50;
const TOP_PICKS: usize = 10;
// Each search scores this many times `shortlist_size` nearest clubs, if they're cached.
const SCORED_PER_SHORTLISTED: usize = 2;

/// Most club details one request fetches from uma.moe by default.
pub const DEFAULT_FETCH_BUDGET: usize = 10;

/// How each request picks and loads clubs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSettings {
    /// Clubs shortlisted per request, from `SHORTLIST_SIZE`.
    pub shortlist_size: usize,
    /// Most club details fetched from uma.moe while the visitor waits, from `REQUEST_FETCH_BUDGET`.
    pub fetch_budget: usize,
}

impl Default for MatchSettings {
    fn default() -> Self {
        Self {
            shortlist_size: DEFAULT_SHORTLIST_SIZE,
            fetch_budget: DEFAULT_FETCH_BUDGET,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    pace: Option<String>,
    trainer_id: Option<String>,
    balance: Option<String>,
    balance_weight: Option<String>,
    include_closed: Option<String>,
    include_full: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Recommendations {
    trainer: TrainerView,
    options: OptionsView,
    updated_at: String,
    month: Option<String>,
    coverage: Coverage,
    #[serde(skip)]
    depth: SearchDepth,
    total: usize,
    clubs: Vec<Recommendation>,
    your_club: Option<YourClub>,
}

#[derive(Debug, Serialize)]
struct YourClub {
    #[serde(flatten)]
    club: Recommendation,
    result_rank: usize,
}

#[derive(Debug, Serialize)]
struct Coverage {
    considered: usize,
    shortlisted: usize,
    scored: usize,
    pending: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct SearchDepth {
    shortlist_rank: Option<u32>,
    pick_rank: Option<u32>,
    pool_bottom: bool,
}

#[derive(Debug, Serialize)]
struct TrainerView {
    trainer_id: Option<u64>,
    pace_per_day: f64,
}

#[derive(Debug, Serialize)]
struct OptionsView {
    balance_weight: Option<f64>,
    include_closed: bool,
    include_full: bool,
    limit: u32,
}

#[derive(Debug, Serialize)]
struct Recommendation {
    circle_id: u64,
    name: String,
    url: String,
    tier: Option<&'static str>,
    join_style: Option<&'static str>,
    policy: Option<u32>,
    member_count: Option<u32>,
    open_spots: Option<u32>,
    monthly_rank: Option<u32>,
    last_month_rank: Option<u32>,
    comment: Option<String>,
    your_place: u32,
    club_size: u32,
    ranked_members: usize,
    median_per_day: f64,
    fit: f64,
    score: f64,
    balance: Option<BalanceView>,
}

#[derive(Debug, Serialize)]
struct BalanceView {
    score: u8,
    gini: f64,
    passengers: usize,
    counted: usize,
    bottom_quarter_per_day: f64,
    top3_share: f64,
}

/// `GET /api/v1/recommendations`: runs [`search`] and logs it.
pub(crate) async fn recommendations(
    state: State<AppState>,
    visitor: Extension<Visitor>,
    RawQuery(query): RawQuery,
    params: Result<Query<Params>, QueryRejection>,
) -> Result<Json<Recommendations>, ApiError> {
    let started = Instant::now();
    let result = search(state, visitor, params).await;
    log_search(query.as_deref(), &result, started.elapsed());
    result
}

fn record_depth(depth: SearchDepth) {
    if let Some(rank) = depth.shortlist_rank {
        metrics::histogram!("search_shortlist_worst_rank").record(f64::from(rank));
    }
    if let Some(rank) = depth.pick_rank {
        metrics::histogram!("search_pick_worst_rank").record(f64::from(rank));
    }
    if depth.pool_bottom {
        metrics::counter!("search_pool_bottom_total").increment(1);
    }
}

/// Logs one line per search with its query string and outcome, so it can be found and rerun.
fn log_search(
    query: Option<&str>,
    result: &Result<Json<Recommendations>, ApiError>,
    took: Duration,
) {
    let duration_ms = u64::try_from(took.as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(Json(found)) => {
            record_depth(found.depth);
            let top_clubs: Vec<u64> = found.clubs.iter().take(10).map(|c| c.circle_id).collect();
            tracing::info!(
                query,
                status = 200,
                trainer_id = found.trainer.trainer_id,
                pace_per_day = found.trainer.pace_per_day,
                month = found.month.as_deref(),
                considered = found.coverage.considered,
                shortlisted = found.coverage.shortlisted,
                scored = found.coverage.scored,
                pending = found.coverage.pending,
                shortlist_rank = found.depth.shortlist_rank,
                pick_rank = found.depth.pick_rank,
                pool_bottom = found.depth.pool_bottom,
                total = found.total,
                top_clubs = ?top_clubs,
                duration_ms,
                "search"
            );
        }
        Err(error) if error.status.is_server_error() => tracing::warn!(
            query,
            status = error.status.as_u16(),
            error = %error.message,
            duration_ms,
            "search failed"
        ),
        Err(error) => tracing::info!(
            query,
            status = error.status.as_u16(),
            error = %error.message,
            duration_ms,
            "search failed"
        ),
    }
}

/// Answers `GET /api/v1/recommendations`; [`recommendations`] wraps it to log the search.
async fn search(
    State(state): State<AppState>,
    Extension(visitor): Extension<Visitor>,
    params: Result<Query<Params>, QueryRejection>,
) -> Result<Json<Recommendations>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::bad_request("invalid query string"))?;
    let pace = blank_to_none(&params.pace);
    let trainer_id = blank_to_none(&params.trainer_id);
    let source = match (pace, trainer_id) {
        (Some(_), Some(_)) => {
            return Err(ApiError::bad_request(
                "use either pace or trainer_id, not both",
            ))
        }
        (None, None) => return Err(ApiError::bad_request("pace or trainer_id is required")),
        (Some(value), None) => PaceSource::Given(parse_number("pace", value)?),
        (None, Some(value)) => PaceSource::Trainer(
            value
                .parse::<u64>()
                .map_err(|_| ApiError::bad_request("trainer_id must be a trainer ID"))?,
        ),
    };
    let balance = parse_bool("balance", &params.balance)?;
    let weight = blank_to_none(&params.balance_weight)
        .map(|value| parse_number("balance_weight", value))
        .transpose()?;
    let balance_weight = match (balance, weight) {
        (Some(false), Some(_)) => {
            return Err(ApiError::bad_request(
                "balance_weight can't be used with balance=false",
            ))
        }
        (_, Some(weight)) => Some(weight),
        (Some(true), None) => Some(DEFAULT_BALANCE_WEIGHT),
        _ => None,
    };
    let include_closed = parse_bool("include_closed", &params.include_closed)?.unwrap_or(false);
    let include_full = parse_bool("include_full", &params.include_full)?.unwrap_or(false);
    let filter = ShortlistFilter {
        include_closed,
        include_full,
    };
    let limit = match blank_to_none(&params.limit) {
        None => DEFAULT_LIMIT,
        Some(value) => value
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| ApiError::bad_request("limit must be a positive whole number"))?,
    };

    let mut options = ScoringOptions::default().with_include_closed(include_closed);
    options.balance_weight = balance_weight;
    let (updated_at, month) = match state.cache.snapshot() {
        Some(snapshot) => (snapshot.scanned_at, snapshot.month.clone()),
        None => state.details.listing().ok_or(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "no scan results yet; try again shortly",
        ))?,
    };
    let (pace, trainer_id, own_club_id) = match source {
        PaceSource::Given(pace) => (pace, None, None),
        PaceSource::Trainer(id) => {
            let period = state.details.period();
            if !state.paces.is_cached(id, period) {
                state.limits.check_lookup(visitor)?;
            }
            let found = state.paces.pace(id, period).await.map_err(pace_error)?;
            (found.pace, Some(id), found.circle_id)
        }
    };
    score_clubs(pace, &[], &options).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let (mut clubs, coverage, mut depth) =
        load_clubs(&state.details, state.matching, pace, filter).await;
    if !include_full {
        let pool = state
            .details
            .pool()
            .read()
            .unwrap_or_else(|e| e.into_inner());
        clubs.retain(|club| pool.is_joinable(club.circle_id));
    }
    clubs.retain(|club| Some(club.circle_id) != own_club_id);
    let scored =
        score_clubs(pace, &clubs, &options).map_err(|e| ApiError::bad_request(e.to_string()))?;
    depth.pick_rank = scored
        .iter()
        .filter(|s| s.club.open_spots() != Some(0))
        .take(TOP_PICKS)
        .filter_map(|s| s.club.monthly_rank)
        .max();
    let your_club = match (trainer_id, own_club_id) {
        (Some(trainer_id), Some(circle_id)) => {
            own_club(
                &state.details,
                circle_id,
                trainer_id,
                pace,
                &options,
                &scored,
            )
            .await
        }
        _ => None,
    };

    Ok(Json(Recommendations {
        trainer: TrainerView {
            trainer_id,
            pace_per_day: pace,
        },
        options: OptionsView {
            balance_weight,
            include_closed,
            include_full,
            limit,
        },
        updated_at: timestamp(updated_at),
        month,
        coverage,
        depth,
        total: scored.len(),
        clubs: scored
            .iter()
            .take(limit as usize)
            .map(recommendation)
            .collect(),
        your_club,
    }))
}

// The trainer's club scored as if they were joining it: without their own member row and one
// member smaller. Closed clubs count, since they're already in it.
async fn own_club(
    details: &ClubDetails,
    circle_id: u64,
    trainer_id: u64,
    pace: f64,
    options: &ScoringOptions,
    scored: &[ScoredClub<'_>],
) -> Option<YourClub> {
    let loaded = details
        .get(circle_id, Priority::Interactive)
        .await
        .inspect_err(|error| {
            tracing::warn!(
                trainer_id,
                circle_id,
                origin = error.origin(),
                %error,
                "trainer's club lookup failed"
            );
        })
        .ok()?;
    let mut club = Club::clone(&loaded);
    let before = club.members.len();
    club.members.retain(|member| member.viewer_id != trainer_id);
    if club.members.len() < before {
        club.member_count = club.member_count.map(|n| n.saturating_sub(1));
    }
    let options = options.with_include_closed(true);
    let own = score_club(pace, &club, &options).ok()??;
    let mut view = recommendation(&own);
    view.member_count = loaded.member_count;
    view.open_spots = loaded.open_spots();
    Some(YourClub {
        club: view,
        result_rank: scored.iter().filter(|s| s.score < own.score).count() + 1,
    })
}

// Every cached club in the pool, the uncached shortlisted clubs current in Redis, and up to
// `fetch_budget` of the rest from uma.moe, nearest first. The other misses are queued for the
// background loader.
async fn load_clubs(
    details: &ClubDetails,
    settings: MatchSettings,
    pace: f64,
    filter: ShortlistFilter,
) -> (Vec<Club>, Coverage, SearchDepth) {
    let (nearest, slowest, considered) = {
        let pool = details.pool().read().unwrap_or_else(|e| e.into_inner());
        let nearest = pool.shortlist_matching(
            TARGET_MEDIAN_RATIO * pace,
            settings.shortlist_size * SCORED_PER_SHORTLISTED,
            filter,
        );
        (nearest, pool.slowest(filter), pool.eligible(filter))
    };
    let shortlist = &nearest[..settings.shortlist_size.min(nearest.len())];
    let depth = SearchDepth {
        shortlist_rank: shortlist
            .iter()
            .filter_map(|&id| details.listed_rank(id))
            .max(),
        pick_rank: None,
        pool_bottom: slowest.is_some_and(|id| shortlist.contains(&id)),
    };
    let mut loaded: HashMap<u64, Arc<Club>> = HashMap::new();
    for &circle_id in &nearest {
        if let Some(club) = details.cached(circle_id).await {
            loaded.insert(circle_id, club);
        }
    }
    let uncached: Vec<u64> = nearest
        .iter()
        .copied()
        .filter(|id| !loaded.contains_key(id))
        .collect();
    for club in details.load_shared(&uncached).await {
        loaded.insert(club.circle_id, club);
    }

    let misses: Vec<u64> = shortlist
        .iter()
        .copied()
        .filter(|id| !loaded.contains_key(id))
        .collect();
    let (fetch, later) = misses.split_at(settings.fetch_budget.min(misses.len()));
    let mut fetches = JoinSet::new();
    for &circle_id in fetch {
        let details = details.clone();
        fetches.spawn(async move {
            (
                circle_id,
                details.get(circle_id, Priority::Interactive).await,
            )
        });
    }
    let mut failed = Vec::new();
    while let Some(joined) = fetches.join_next().await {
        match joined {
            Ok((circle_id, Ok(club))) => {
                loaded.insert(circle_id, club);
            }
            Ok((circle_id, Err(_))) => failed.push(circle_id),
            Err(_) => {}
        }
    }
    for &circle_id in later.iter().chain(&failed) {
        details.queue(circle_id);
    }

    let pending = shortlist
        .iter()
        .filter(|id| !loaded.contains_key(id))
        .count();
    let mut clubs: Vec<Club> = loaded.into_values().map(|c| Club::clone(&c)).collect();
    clubs.sort_by_key(|c| (c.monthly_rank.unwrap_or(u32::MAX), c.circle_id));
    let coverage = Coverage {
        considered,
        shortlisted: shortlist.len(),
        scored: clubs.len(),
        pending,
    };
    (clubs, coverage, depth)
}

enum PaceSource {
    Given(f64),
    Trainer(u64),
}

fn parse_bool(name: &str, value: &Option<String>) -> Result<Option<bool>, ApiError> {
    match blank_to_none(value) {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(ApiError::bad_request(format!(
            "{name} must be true or false"
        ))),
    }
}

fn blank_to_none(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

fn pace_error(error: PaceError) -> ApiError {
    let status = match error {
        PaceError::NotFound => StatusCode::NOT_FOUND,
        PaceError::NotEnoughData => StatusCode::UNPROCESSABLE_ENTITY,
        PaceError::Upstream(_) => StatusCode::BAD_GATEWAY,
    };
    ApiError::new(status, error.to_string())
}

fn parse_number(name: &str, value: &str) -> Result<f64, ApiError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| ApiError::bad_request(format!("{name} must be a number")))
}

fn recommendation(scored: &ScoredClub<'_>) -> Recommendation {
    let club = scored.club;
    Recommendation {
        circle_id: club.circle_id,
        name: club.name.clone(),
        url: format!("{CLUB_URL}{}", club.circle_id),
        tier: club.tier(),
        join_style: club.join_style.and_then(|style| match style {
            JoinStyle::Open => Some("open"),
            JoinStyle::Approval => Some("approval"),
            JoinStyle::Closed => Some("closed"),
            _ => None,
        }),
        policy: club.policy,
        member_count: club.member_count,
        open_spots: club.open_spots(),
        monthly_rank: club.monthly_rank,
        last_month_rank: club.last_month_rank,
        comment: club.comment.clone(),
        your_place: scored.your_place,
        club_size: scored.club_size,
        ranked_members: scored.ranked_members,
        median_per_day: scored.median_per_day,
        fit: scored.fit,
        score: scored.score,
        balance: scored.balance.map(balance_view),
    }
}

fn balance_view(balance: Balance) -> BalanceView {
    BalanceView {
        score: balance.score,
        gini: balance.gini,
        passengers: balance.passengers,
        counted: balance.counted,
        bottom_quarter_per_day: balance.bottom_quarter_per_day,
        top3_share: balance.top3_share,
    }
}
