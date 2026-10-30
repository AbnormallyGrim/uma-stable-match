//! Last month's paces, kept as priors for blending club medians early in the month.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use uma_stable_match_scoring::{daily_pace, Club};

use crate::calibration::club_median;
use crate::pool::Period;
use crate::shared::SharedCache;

/// Days of data in a club from which its median no longer blends in last month's paces.
pub const CLUB_BLEND_DAYS: usize = 6;

/// How long club medians lean on last month's paces: `k` in the blend.
pub const CLUB_PRIOR_K: f64 = 1.0;

// Long enough to cover the blend window after the month changes.
const KEEP_PRIORS: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Members' and clubs' paces in one game month, used as priors in the next.
#[derive(Debug, Default)]
pub(crate) struct Priors {
    month: RwLock<Month>,
}

#[derive(Debug, Default)]
struct Month {
    label: Option<String>,
    members: HashMap<u64, f64>,
    clubs: HashMap<u64, f64>,
}

impl Priors {
    /// Keeps the paces in `club`, a club's details from an earlier game month, as priors.
    pub(crate) async fn capture(&self, shared: &SharedCache, circle_id: u64, club: &Club) {
        let Some(label) = data_period(club).map(month_label) else {
            return;
        };
        let paces: Vec<(u64, f64)> = club
            .members
            .iter()
            .filter_map(|m| Some((m.viewer_id, daily_pace(&m.daily_fans)?.max(0.0))))
            .collect();
        let median = club_median(club);
        {
            let mut month = self.month_for(&label);
            month.members.extend(paces.iter().copied());
            if let Some(median) = median {
                month.clubs.insert(circle_id, median);
            }
        }
        shared.store_priors(&label, &paces, KEEP_PRIORS).await;
        if let Some(median) = median {
            shared
                .store_medians(&label, &[(circle_id, median)], KEEP_PRIORS)
                .await;
        }
    }

    /// Keeps clubs' exact medians from the month before as their club priors.
    pub(crate) fn set_club_medians(
        &self,
        period: Period,
        medians: impl IntoIterator<Item = (u64, f64)>,
    ) {
        self.month_for(&month_label(period)).clubs.extend(medians);
    }

    /// Gives each member of `club`, this month's details, a prior from the month before: their own
    /// pace, else the club's median, else `fallback`.
    pub(crate) async fn attach(
        &self,
        shared: &SharedCache,
        circle_id: u64,
        club: &mut Club,
        fallback: Option<f64>,
    ) {
        let Some(label) = data_period(club).map(month_before).map(month_label) else {
            return;
        };
        let (mut members, mut club_prior) = {
            let month = self.month_for_read(&label);
            let members: Vec<Option<f64>> = club
                .members
                .iter()
                .map(|m| month.and_then(|month| month.members.get(&m.viewer_id).copied()))
                .collect();
            let club_prior = month.and_then(|month| month.clubs.get(&circle_id).copied());
            (members, club_prior)
        };
        let missing: Vec<u64> = club
            .members
            .iter()
            .zip(&members)
            .filter(|(_, prior)| prior.is_none())
            .map(|(m, _)| m.viewer_id)
            .collect();
        if !missing.is_empty() {
            let mut stored = shared.priors(&label, &missing).await.into_iter();
            for prior in members.iter_mut().filter(|prior| prior.is_none()) {
                *prior = stored.next().flatten();
            }
        }
        if club_prior.is_none() {
            club_prior = shared
                .medians_of(&label, &[circle_id])
                .await
                .into_iter()
                .next()
                .flatten();
        }
        let club_prior = club_prior.or(fallback);
        for (member, prior) in club.members.iter_mut().zip(members) {
            member.prior = prior.or(club_prior);
        }
    }

    fn month_for(&self, label: &str) -> std::sync::RwLockWriteGuard<'_, Month> {
        let mut month = self.month.write().unwrap_or_else(|e| e.into_inner());
        // Only one month's priors are needed at a time.
        if month.label.as_deref() != Some(label) {
            *month = Month {
                label: Some(label.to_owned()),
                ..Month::default()
            };
        }
        month
    }

    fn month_for_read(&self, label: &str) -> MonthRead<'_> {
        let month = self.month.read().unwrap_or_else(|e| e.into_inner());
        MonthRead {
            matches: month.label.as_deref() == Some(label),
            month,
        }
    }
}

struct MonthRead<'a> {
    month: std::sync::RwLockReadGuard<'a, Month>,
    matches: bool,
}

impl MonthRead<'_> {
    fn and_then<T>(&self, f: impl FnOnce(&Month) -> Option<T>) -> Option<T> {
        if self.matches {
            f(&self.month)
        } else {
            None
        }
    }
}

/// The latest game month among a club's member snapshots.
pub(crate) fn data_period(club: &Club) -> Option<Period> {
    club.members
        .iter()
        .filter_map(|m| m.year.zip(m.month))
        .max()
}

/// The most days of data any member of `club` has.
pub(crate) fn data_days(club: &Club) -> usize {
    club.members
        .iter()
        .map(|m| m.daily_fans.iter().filter(|v| **v != 0).count())
        .max()
        .unwrap_or(0)
}

/// The game month before `period`.
pub(crate) fn month_before((year, month): Period) -> Period {
    if month <= 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

pub(crate) fn month_label((year, month): Period) -> String {
    format!("{year:04}-{month:02}")
}

#[cfg(test)]
mod tests;
