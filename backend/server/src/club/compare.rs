//! `GET /api/v1/clubs/{id}/compare`: the club's members and where each of up to 5 trainers would sit.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uma_stable_match_scoring::{blended_median, Club};

use super::{
    charge, members, previous_fans, trainer_view, MemberView, PreviousFans, TrainerView,
    PREVIOUS_MONTH_BELOW_DAYS,
};
use crate::api_error::ApiError;
use crate::app::{timestamp, AppState};
use crate::applications::{mark, Applications};
use crate::limits::Visitor;
use crate::priors::{data_days, data_period, month_before, month_label, CLUB_PRIOR_K};
use crate::recommendations::{club_info, without_trainer, ClubInfo};

/// Most trainers one comparison places.
pub(crate) const MAX_TRAINERS: usize = 5;

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    trainer_ids: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ComparePage {
    club: ClubInfo,
    month: Option<String>,
    updated_at: Option<String>,
    median_per_day: Option<f64>,
    members: Vec<MemberView>,
    trainers: Vec<TrainerView>,
    previous: Option<PreviousFans>,
}

// The trainer IDs in `raw`, comma separated, without repeats and in order.
fn parse_ids(raw: Option<&str>) -> Result<Vec<u64>, ApiError> {
    let invalid = || ApiError::bad_request("trainer_ids must be 1 to 5 trainer IDs");
    let mut ids: Vec<u64> = Vec::new();
    for part in raw.unwrap_or_default().split(',') {
        let id = part.trim().parse::<u64>().map_err(|_| invalid())?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() > MAX_TRAINERS {
        return Err(invalid());
    }
    Ok(ids)
}

/// Serves `GET /api/v1/clubs/{id}/compare` from loaded clubs only. Each trainer is placed among the
/// members alone, as if they were the only one joining.
pub(crate) async fn compare(
    State(state): State<AppState>,
    Extension(visitor): Extension<Visitor>,
    applications: Option<Extension<Applications>>,
    Path(id): Path<String>,
    params: Result<Query<Params>, QueryRejection>,
) -> Result<Json<ComparePage>, ApiError> {
    let Query(params) = params.map_err(|_| ApiError::bad_request("invalid query string"))?;
    let circle_id = id
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request("club ID must be a club ID"))?;
    let trainer_ids = parse_ids(params.trainer_ids.as_deref())?;
    let loaded = state.details.loaded(circle_id).await.ok_or(ApiError::new(
        StatusCode::NOT_FOUND,
        "club not loaded yet; try again later",
    ))?;

    // The trainers' own rows are left out, so a trainer already in the club isn't counted twice.
    let club = trainer_ids
        .iter()
        .fold(Club::clone(&loaded), |club, id| without_trainer(&club, *id));
    // Every lookup is counted before the first fetch, so a visitor over the limit gets nothing.
    for id in &trainer_ids {
        charge(&state, visitor, *id)?;
    }
    let mut trainers = Vec::with_capacity(trainer_ids.len());
    for id in &trainer_ids {
        let in_club = loaded.members.iter().any(|m| m.viewer_id == *id);
        trainers.push(trainer_view(&state, *id, &club, in_club).await?);
    }

    let mut info = club_info(&club);
    info.member_count = loaded.member_count;
    info.open_spots = loaded.open_spots();
    mark(&applications, [&mut info]).await;
    let (ids, members): (Vec<u64>, Vec<MemberView>) = members(&club).into_iter().unzip();
    let previous = match data_period(&loaded) {
        Some(period) if data_days(&loaded) < PREVIOUS_MONTH_BELOW_DAYS => {
            previous_fans(
                &state,
                visitor,
                circle_id,
                month_before(period),
                &ids,
                &trainer_ids,
            )
            .await
        }
        _ => None,
    };

    Ok(Json(ComparePage {
        club: info,
        month: data_period(&club).map(month_label),
        updated_at: state
            .details
            .last_updated(circle_id)
            .await
            .map(|at| timestamp(at.into())),
        median_per_day: blended_median(&club.members, CLUB_PRIOR_K),
        members,
        trainers,
        previous,
    }))
}

#[cfg(test)]
mod tests {
    use super::parse_ids;

    #[test]
    fn keeps_order_and_drops_repeats() {
        assert_eq!(parse_ids(Some("3, 1,3,2")).ok(), Some(vec![3, 1, 2]));
    }

    #[test]
    fn needs_one_to_five_trainer_ids() {
        for raw in [None, Some(""), Some("1,,2"), Some("x"), Some("1,2,3,4,5,6")] {
            assert!(parse_ids(raw).is_err(), "{raw:?}");
        }
        assert_eq!(
            parse_ids(Some("1,2,3,4,5,5")).ok().map(|ids| ids.len()),
            Some(5)
        );
    }
}
