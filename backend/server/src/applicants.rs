//! Lets Discord applications show a trainer's pace and fans.

use uma_stable_match_discord::{Alltime, Applicant, ApplicantLookup, LookupError};

use uma_stable_match_umamoe_client::ProfileFanHistory;

use crate::pace::{PaceError, PaceLookup};
use crate::source::Source;

/// An applicant lookup that reads the trainer's profile from `source` and their pace and fans
/// this month from `paces`. A trainer the source doesn't know is `None`. A trainer without enough
/// data for a pace still applies, without one.
pub fn applicant_lookup(source: Source, paces: PaceLookup) -> ApplicantLookup {
    ApplicantLookup::new(move |trainer_id| {
        let (source, paces) = (source.clone(), paces.clone());
        async move {
            let profile = match source.profile(trainer_id).await {
                Ok(profile) => profile,
                Err(error) if error.is_not_found() => return Ok(None),
                Err(error) => {
                    tracing::warn!(
                        origin = error.origin(),
                        "could not load an applicant's profile for Discord"
                    );
                    return Err(LookupError);
                }
            };
            let pace = match paces.blended(trainer_id, None).await {
                Ok(found) => Some(found.pace),
                Err(PaceError::NotFound | PaceError::NotEnoughData) => None,
                Err(_) => {
                    tracing::warn!("could not load an applicant's pace for Discord");
                    None
                }
            };
            let fans = paces
                .daily_fans(trainer_id, None)
                .await
                .ok()
                .flatten()
                .and_then(|daily| fans_this_month(&daily));
            Ok(Some(Applicant {
                name: profile.trainer.name.clone(),
                club_name: profile.circle.map(|circle| circle.name),
                pace_per_day: pace,
                fans_this_month: fans,
                alltime: alltime(profile.fan_history),
                leader_chara_dress_id: profile.trainer.leader_chara_dress_id,
            }))
        }
    })
}

/// The all-time fans from a profile's fan history, if it has them.
pub(crate) fn alltime(history: Option<ProfileFanHistory>) -> Option<Alltime> {
    let alltime = history?.alltime?;
    Some(Alltime {
        total_fans: alltime.total_fans,
        active_days: alltime.active_days,
        avg_day: alltime.avg_day,
        avg_week: alltime.avg_week,
        avg_month: alltime.avg_month,
        rank: alltime.rank_total_fans,
    })
}

/// Fans gained this month from the cumulative daily series: the last day's total less the first's,
/// skipping days without data. `None` with fewer than two days.
pub(crate) fn fans_this_month(daily_fans: &[i64]) -> Option<i64> {
    let mut days = daily_fans.iter().filter(|v| **v != 0).map(|v| v.abs());
    let first = days.next()?;
    let last = days.next_back()?;
    Some(last - first)
}

#[cfg(test)]
mod tests;
