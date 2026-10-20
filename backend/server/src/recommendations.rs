//! `GET /api/v1/recommendations`: scores the cached clubs for a pace or a trainer.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uma_stable_match_scoring::{
    score_clubs, Balance, JoinStyle, ScoredClub, ScoringOptions, DEFAULT_BALANCE_WEIGHT,
};

use crate::api_error::ApiError;
use crate::app::timestamp;
use crate::app::AppState;
use crate::limits::Visitor;
use crate::pace::PaceError;

const CLUB_URL: &str = "https://uma.moe/circles/";
const DEFAULT_LIMIT: u32 = 50;

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    pace: Option<String>,
    trainer_id: Option<String>,
    balance: Option<String>,
    balance_weight: Option<String>,
    include_closed: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Recommendations {
    trainer: TrainerView,
    options: OptionsView,
    updated_at: String,
    total: usize,
    clubs: Vec<Recommendation>,
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

pub(crate) async fn recommendations(
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
    let snapshot = state.cache.snapshot().ok_or(ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "no scan results yet; try again shortly",
    ))?;
    let (pace, trainer_id) = match source {
        PaceSource::Given(pace) => (pace, None),
        PaceSource::Trainer(id) => {
            if !state.paces.is_cached(id) {
                state.limits.check_lookup(visitor)?;
            }
            (state.paces.pace(id).await.map_err(pace_error)?, Some(id))
        }
    };
    let scored = score_clubs(pace, &snapshot.clubs, &options)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    Ok(Json(Recommendations {
        trainer: TrainerView {
            trainer_id,
            pace_per_day: pace,
        },
        options: OptionsView {
            balance_weight,
            include_closed,
            limit,
        },
        updated_at: timestamp(snapshot.scanned_at),
        total: scored.len(),
        clubs: scored
            .iter()
            .take(limit as usize)
            .map(recommendation)
            .collect(),
    }))
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
