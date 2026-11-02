//! `GET /api/v1/clubs/{id}`: a loaded club's members, and where a trainer would sit among them.

use std::collections::HashMap;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uma_stable_match_scoring::{
    blended_median, blended_pace, current_members, score_club, Club, ScoringOptions,
};

use crate::api_error::ApiError;
use crate::app::{timestamp, AppState};
use crate::limits::Visitor;
use crate::pool::Period;
use crate::priors::{data_days, data_period, month_before, month_label, CLUB_PRIOR_K};
use crate::recommendations::{blank_to_none, club_info, pace_error, without_trainer, ClubInfo};

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    trainer_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClubPage {
    club: ClubInfo,
    month: Option<String>,
    updated_at: Option<String>,
    median_per_day: Option<f64>,
    members: Vec<MemberView>,
    trainer: Option<TrainerView>,
    previous: Option<PreviousView>,
}

// Last month's daily fans, while this month has too few days for a chart. `members` lines up with
// the page's members, with `None` for those who weren't in the club.
#[derive(Debug, Serialize)]
struct PreviousView {
    month: String,
    members: Vec<Option<Vec<i64>>>,
    trainer: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
struct MemberView {
    name: Option<String>,
    pace_per_day: Option<f64>,
    daily_fans: Vec<i64>,
}

#[derive(Debug, Serialize)]
struct TrainerView {
    trainer_id: u64,
    pace_per_day: f64,
    daily_fans: Option<Vec<i64>>,
    in_club: bool,
    your_place: Option<u32>,
    club_size: Option<u32>,
}

/// Serves `GET /api/v1/clubs/{id}` from loaded clubs only, so it never waits on the source for a
/// club.
pub(crate) async fn club(
    State(state): State<AppState>,
    Extension(visitor): Extension<Visitor>,
    Path(id): Path<String>,
    params: Result<Query<Params>, QueryRejection>,
) -> Result<Json<ClubPage>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::bad_request("invalid query string"))?;
    let circle_id = id
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request("club ID must be a club ID"))?;
    let trainer_id = blank_to_none(&params.trainer_id)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| ApiError::bad_request("trainer_id must be a trainer ID"))
        })
        .transpose()?;
    let loaded = state.details.loaded(circle_id).await.ok_or(ApiError::new(
        StatusCode::NOT_FOUND,
        "club not loaded yet; try again later",
    ))?;

    let in_club = trainer_id.is_some_and(|id| loaded.members.iter().any(|m| m.viewer_id == id));
    let club = match trainer_id {
        Some(id) => without_trainer(&loaded, id),
        None => Club::clone(&loaded),
    };
    let trainer = match trainer_id {
        Some(id) => Some(trainer_view(&state, visitor, id, &club, in_club).await?),
        None => None,
    };
    let mut info = club_info(&club);
    info.member_count = loaded.member_count;
    info.open_spots = loaded.open_spots();
    let (ids, members): (Vec<u64>, Vec<MemberView>) = members(&club).into_iter().unzip();
    let previous = match data_period(&loaded) {
        Some(period) if data_days(&loaded) < PREVIOUS_MONTH_BELOW_DAYS => {
            previous_view(
                &state,
                visitor,
                circle_id,
                month_before(period),
                &ids,
                trainer_id,
            )
            .await
        }
        _ => None,
    };

    Ok(Json(ClubPage {
        club: info,
        month: data_period(&club).map(month_label),
        updated_at: state
            .details
            .last_updated(circle_id)
            .await
            .map(|at| timestamp(at.into())),
        median_per_day: blended_median(&club.members, CLUB_PRIOR_K),
        members,
        trainer,
        previous,
    }))
}

// Days of this month's data below which the page also gets the club's previous month. The daily
// chart leaves out the day under way and needs two gains, so it takes 4 days.
const PREVIOUS_MONTH_BELOW_DAYS: usize = 4;

// The club's previous month for `ids`, its current members, and the trainer. `None` when the
// visitor is over their lookup limit for an uncached month, or the fetch fails.
async fn previous_view(
    state: &AppState,
    visitor: Visitor,
    circle_id: u64,
    period: Period,
    ids: &[u64],
    trainer_id: Option<u64>,
) -> Option<PreviousView> {
    if !state.details.has_previous_month(circle_id, period) {
        state.limits.check_lookup(visitor).ok()?;
    }
    let club = state
        .details
        .previous_month(circle_id, period)
        .await
        .inspect_err(|error| {
            tracing::warn!(circle_id, origin = error.origin(), %error, "previous month lookup failed");
        })
        .ok()?;
    let fans: HashMap<u64, Vec<i64>> = club
        .members
        .iter()
        .filter(|m| m.year.zip(m.month).is_none_or(|p| p == period))
        .map(|m| (m.viewer_id, m.daily_fans.iter().map(|v| v.abs()).collect()))
        .collect();
    let trainer = match trainer_id {
        Some(id) => state
            .paces
            .daily_fans(id, Some(period))
            .await
            .ok()
            .flatten()
            .map(|fans| fans.iter().map(|v| v.abs()).collect()),
        None => None,
    };
    Some(PreviousView {
        month: month_label(period),
        members: ids.iter().map(|id| fans.get(id).cloned()).collect(),
        trainer,
    })
}

// Current members with their trainer IDs, fastest first; members too new to have a pace come last.
fn members(club: &Club) -> Vec<(u64, MemberView)> {
    let mut members: Vec<(u64, MemberView)> = current_members(&club.members)
        .into_iter()
        .map(|m| {
            let view = MemberView {
                name: m.name.clone(),
                pace_per_day: blended_pace(&m.daily_fans, m.prior, CLUB_PRIOR_K)
                    .map(|p| p.max(0.0)),
                daily_fans: m.daily_fans.iter().map(|v| v.abs()).collect(),
            };
            (m.viewer_id, view)
        })
        .collect();
    members.sort_by(|(_, a), (_, b)| match (a.pace_per_day, b.pace_per_day) {
        (Some(a), Some(b)) => b.total_cmp(&a),
        (a, b) => b.is_some().cmp(&a.is_some()),
    });
    members
}

// The trainer's pace and fans, and their place in `club`, which no longer has their own row.
async fn trainer_view(
    state: &AppState,
    visitor: Visitor,
    trainer_id: u64,
    club: &Club,
    in_club: bool,
) -> Result<TrainerView, ApiError> {
    if !state.paces.is_cached(trainer_id, None) {
        state.limits.check_lookup(visitor)?;
    }
    let found = state
        .paces
        .blended(trainer_id, state.details.prior_period())
        .await
        .map_err(pace_error)?;
    let daily_fans = state
        .paces
        .daily_fans(trainer_id, state.details.period())
        .await
        .ok()
        .flatten()
        .map(|fans| fans.iter().map(|v| v.abs()).collect());
    let options = ScoringOptions::default()
        .with_include_closed(true)
        .with_prior_k(CLUB_PRIOR_K);
    let scored = score_club(found.pace, club, &options).ok().flatten();
    Ok(TrainerView {
        trainer_id,
        pace_per_day: found.pace,
        daily_fans,
        in_club,
        your_place: scored.as_ref().map(|s| s.your_place),
        club_size: scored.as_ref().map(|s| s.club_size),
    })
}

#[cfg(test)]
mod tests;
