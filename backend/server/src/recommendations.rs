use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uma_stable_match_scoring::{score_clubs, Balance, JoinStyle, ScoredClub, ScoringOptions};

use crate::app::timestamp;
use crate::cache::Cache;

const CLUB_URL: &str = "https://uma.moe/circles/";

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    pace: Option<String>,
    balance_weight: Option<String>,
    include_closed: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Recommendations {
    scanned_at: String,
    pace: f64,
    balance_weight: Option<f64>,
    include_closed: bool,
    results: Vec<Recommendation>,
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

pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

pub(crate) async fn recommendations(
    State(cache): State<Cache>,
    Query(params): Query<Params>,
) -> Result<Json<Recommendations>, ApiError> {
    let pace = match params.pace.as_deref().map(str::trim) {
        None | Some("") => return Err(ApiError::bad_request("pace is required")),
        Some(value) => parse_number("pace", value)?,
    };
    let balance_weight = match params.balance_weight.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => Some(parse_number("balance_weight", value)?),
    };
    let include_closed = match params.include_closed.as_deref().map(str::trim) {
        None | Some("") | Some("false") => false,
        Some("true") => true,
        Some(_) => {
            return Err(ApiError::bad_request(
                "include_closed must be true or false",
            ))
        }
    };

    let options = ScoringOptions {
        balance_weight,
        include_closed,
    };
    let snapshot = cache.snapshot().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "no scan results yet; try again shortly".into(),
    })?;
    let scored = score_clubs(pace, &snapshot.clubs, &options)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    Ok(Json(Recommendations {
        scanned_at: timestamp(snapshot.scanned_at),
        pace,
        balance_weight,
        include_closed,
        results: scored.iter().map(recommendation).collect(),
    }))
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
        join_style: club.join_style.map(|style| match style {
            JoinStyle::Open => "open",
            JoinStyle::Approval => "approval",
            JoinStyle::Closed => "closed",
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
