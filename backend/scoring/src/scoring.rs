//! Club scoring: place, fit, balance and ranking.

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::members::{
    blended_pace, current_members, daily_pace, days_with_data, MIN_DAYS_FOR_PACE,
};
use crate::model::{Club, JoinStyle, MemberSnapshot, CLUB_CAP};
use crate::stats::{gini, median, round_half_even};

/// Minimum rankable current members a club needs to be scored.
pub const MIN_RANKED_MEMBERS: usize = 8;

/// Members below this daily gain are passengers.
pub const PASSENGER_FLOOR: f64 = 1_000_000.0;

/// Members below this share of the club median are passengers.
pub const PASSENGER_SHARE_OF_MEDIAN: f64 = 0.25;

/// Default balance weight.
pub const DEFAULT_BALANCE_WEIGHT: f64 = 0.5;

// Fit targets: about 40% of the way down the club, with a median at 85% of your pace.
const IDEAL_POSITION: f64 = 0.4;
const IDEAL_MEDIAN_RATIO: f64 = 0.85;
const MEDIAN_WEIGHT: f64 = 0.3;

/// Options for [`score_clubs`] and [`score_club`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ScoringOptions {
    /// Turns on balance scoring with this weight (0 to 1).
    pub balance_weight: Option<f64>,
    /// Include clubs that aren't accepting members.
    pub include_closed: bool,
}

impl ScoringOptions {
    /// Turns on balance scoring with this weight (0 to 1).
    pub fn with_balance_weight(mut self, weight: f64) -> Self {
        self.balance_weight = Some(weight);
        self
    }

    /// Sets whether to include clubs that aren't accepting members.
    pub fn with_include_closed(mut self, include: bool) -> Self {
        self.include_closed = include;
        self
    }
}

/// Invalid input to [`score_clubs`] or [`score_club`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ScoringError {
    /// Pace must be finite and above zero.
    InvalidPace(f64),
    /// Balance weight must be between 0 and 1.
    InvalidBalanceWeight(f64),
}

impl fmt::Display for ScoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPace(p) => write!(f, "pace must be greater than zero, got {p}"),
            Self::InvalidBalanceWeight(w) => {
                write!(f, "balance weight must be between 0 and 1, got {w}")
            }
        }
    }
}

impl std::error::Error for ScoringError {}

/// How evenly a club's current members contribute.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Balance {
    /// 0 to 100; higher is more even.
    pub score: u8,
    /// Gini coefficient of daily gains (0 = even, 1 = one member does everything).
    pub gini: f64,
    /// Number of passengers.
    pub passengers: usize,
    /// Number of members counted.
    pub counted: usize,
    /// Median daily gain of the lowest-contributing quarter.
    pub bottom_quarter_per_day: f64,
    /// Share of daily gain from the top three members (0 to 1).
    pub top3_share: f64,
}

/// A club's scores for one trainer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ScoredClub<'a> {
    /// The club that was scored.
    pub club: &'a Club,
    /// Median daily gain of current members.
    pub median_per_day: f64,
    /// Your rank among current members, starting at 1.
    pub your_place: u32,
    /// Club size after you join, at most [`CLUB_CAP`].
    pub club_size: u32,
    /// Current members with enough data to rank.
    pub ranked_members: usize,
    /// Fit score; lower is better.
    pub fit: f64,
    /// Balance, when balance scoring is on.
    pub balance: Option<Balance>,
    /// Final score used for ranking; lower is better.
    pub score: f64,
}

/// Balance of a club's current members, or `None` if too few can be counted.
pub fn club_balance(members: &[MemberSnapshot]) -> Option<Balance> {
    // New members without enough data are left out so they aren't counted as passengers.
    let mut rates: Vec<f64> = current_members(members)
        .into_iter()
        .filter(|m| days_with_data(&m.daily_fans) >= MIN_DAYS_FOR_PACE)
        .map(|m| daily_pace(&m.daily_fans).unwrap_or(0.0).max(0.0))
        .collect();
    if rates.len() < MIN_RANKED_MEMBERS {
        return None;
    }
    rates.sort_by(|a, b| b.total_cmp(a));

    let club_median = median(&rates)?;
    let floor = PASSENGER_FLOOR.max(PASSENGER_SHARE_OF_MEDIAN * club_median);
    let passengers = rates.iter().filter(|r| **r < floor).count();
    let g = gini(&rates);
    let quarter = (rates.len() / 4).max(1);
    let bottom_quarter = median(&rates[rates.len() - quarter..])?;
    let total: f64 = rates.iter().sum();
    let top3_share = if total > 0.0 {
        rates.iter().take(3).sum::<f64>() / total
    } else {
        0.0
    };
    let passenger_share = passengers as f64 / rates.len() as f64;
    let score = round_half_even(100.0 * (0.5 * (1.0 - g) + 0.5 * (1.0 - passenger_share)));

    Some(Balance {
        score: score.clamp(0.0, 100.0) as u8,
        gini: g,
        passengers,
        counted: rates.len(),
        bottom_quarter_per_day: bottom_quarter,
        top3_share,
    })
}

/// Median [`blended_pace`] of a club's current members, with `prior` giving each member's prior
/// by trainer ID. `None` with fewer than [`MIN_RANKED_MEMBERS`] paces.
pub fn blended_median(
    members: &[MemberSnapshot],
    prior: impl Fn(u64) -> Option<f64>,
    k: f64,
) -> Option<f64> {
    let rates: Vec<f64> = current_members(members)
        .into_iter()
        .filter_map(|m| blended_pace(&m.daily_fans, prior(m.viewer_id), k))
        .map(|r| r.max(0.0))
        .collect();
    if rates.len() < MIN_RANKED_MEMBERS {
        return None;
    }
    median(&rates)
}

/// Scores one club, or returns `Ok(None)` if the club is left out.
pub fn score_club<'a>(
    pace: f64,
    club: &'a Club,
    options: &ScoringOptions,
) -> Result<Option<ScoredClub<'a>>, ScoringError> {
    validate(pace, options)?;
    Ok(score_valid(pace, club, options))
}

/// Scores and ranks clubs, best first.
pub fn score_clubs<'a>(
    pace: f64,
    clubs: &'a [Club],
    options: &ScoringOptions,
) -> Result<Vec<ScoredClub<'a>>, ScoringError> {
    validate(pace, options)?;
    let mut scored: Vec<ScoredClub<'a>> = clubs
        .iter()
        .filter_map(|club| score_valid(pace, club, options))
        .collect();
    scored.sort_by(|a, b| a.score.total_cmp(&b.score));
    Ok(scored)
}

fn validate(pace: f64, options: &ScoringOptions) -> Result<(), ScoringError> {
    if !(pace.is_finite() && pace > 0.0) {
        return Err(ScoringError::InvalidPace(pace));
    }
    if let Some(w) = options.balance_weight {
        if !(0.0..=1.0).contains(&w) {
            return Err(ScoringError::InvalidBalanceWeight(w));
        }
    }
    Ok(())
}

fn score_valid<'a>(pace: f64, club: &'a Club, options: &ScoringOptions) -> Option<ScoredClub<'a>> {
    if club.join_style == Some(JoinStyle::Closed) && !options.include_closed {
        return None;
    }

    let current = current_members(&club.members);
    let rates: Vec<f64> = current
        .iter()
        .filter_map(|m| daily_pace(&m.daily_fans))
        .map(|r| r.max(0.0))
        .collect();
    if rates.len() < MIN_RANKED_MEMBERS {
        return None;
    }
    let club_median = median(&rates)?;

    // Unrankable new members count towards the size but are placed below you.
    let known_members = club
        .member_count
        .filter(|n| *n > 0)
        .unwrap_or(current.len() as u32);
    let club_size = known_members.min(CLUB_CAP - 1) + 1;
    let faster = rates.iter().filter(|r| **r > pace).count() as u32;
    let your_place = (faster + 1).min(club_size);

    let position = f64::from(your_place) / f64::from(club_size);
    let fit = (position - IDEAL_POSITION).abs()
        + MEDIAN_WEIGHT * (club_median / pace - IDEAL_MEDIAN_RATIO).abs();

    let (balance, score) = match options.balance_weight {
        Some(weight) => {
            let balance = club_balance(&club.members)?;
            let score = fit + weight * (1.0 - f64::from(balance.score) / 100.0);
            (Some(balance), score)
        }
        None => (None, fit),
    };

    Some(ScoredClub {
        club,
        median_per_day: club_median,
        your_place,
        club_size,
        ranked_members: rates.len(),
        fit,
        balance,
        score,
    })
}

#[cfg(test)]
mod tests;
