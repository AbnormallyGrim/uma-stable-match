//! Club scoring: your place, fit, balance and the final ranking.

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::members::{current_members, daily_pace, days_with_data, MIN_DAYS_FOR_PACE};
use crate::model::{Club, JoinStyle, MemberSnapshot, CLUB_CAP};
use crate::stats::{gini, median, round_half_even};

/// Minimum number of rankable current members a club needs to be scored.
pub const MIN_RANKED_MEMBERS: usize = 8;

/// A member gaining less than this many fans per day is a passenger.
pub const PASSENGER_FLOOR: f64 = 1_000_000.0;

/// A member gaining less than this share of the club median is a passenger.
pub const PASSENGER_SHARE_OF_MEDIAN: f64 = 0.25;

/// Default strength of balance scoring.
pub const DEFAULT_BALANCE_WEIGHT: f64 = 0.5;

/// Where in the club you should ideally sit, as a fraction from the top.
const IDEAL_POSITION: f64 = 0.4;
/// Ideal club median as a fraction of your pace.
const IDEAL_MEDIAN_RATIO: f64 = 0.85;
/// How much the median term counts towards fit.
const MEDIAN_WEIGHT: f64 = 0.3;

/// Options for [`score_clubs`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScoringOptions {
    /// Turns on balance scoring with this weight, from 0 to 1. Clubs without
    /// enough data for a balance score are left out when this is set.
    pub balance_weight: Option<f64>,
    /// Include clubs that aren't accepting members.
    pub include_closed: bool,
}

/// Errors returned by [`score_clubs`] and [`score_club`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoringError {
    /// The trainer's pace must be a finite number greater than zero.
    InvalidPace(f64),
    /// The balance weight must be between 0 and 1.
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
pub struct Balance {
    /// 0 to 100. Higher means members contribute more evenly.
    pub score: u8,
    /// Gini coefficient of daily gains: 0 is perfectly even, 1 is one member
    /// doing everything.
    pub gini: f64,
    /// Number of passengers among the counted members.
    pub passengers: usize,
    /// Number of current members counted (those with enough data).
    pub counted: usize,
    /// Median daily gain of the lowest-contributing quarter.
    pub bottom_quarter_per_day: f64,
    /// Share of the club's daily gain from its top three members, from 0 to 1.
    pub top3_share: f64,
}

/// A club with its scores for one trainer.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredClub<'a> {
    /// The club that was scored.
    pub club: &'a Club,
    /// Median daily gain of the club's current members.
    pub median_per_day: f64,
    /// Where you would rank among current members, starting at 1.
    pub your_place: u32,
    /// Club size after you join, including you. At most [`CLUB_CAP`].
    pub club_size: u32,
    /// Number of current members with enough data to rank.
    pub ranked_members: usize,
    /// Fit score. Lower is better.
    pub fit: f64,
    /// Balance, when balance scoring is on.
    pub balance: Option<Balance>,
    /// Final score used for ranking. Lower is better. Equal to `fit` when
    /// balance scoring is off.
    pub score: f64,
}

/// Balance of a club's current members.
///
/// Members with fewer than three days of data are left out, so new recruits
/// aren't counted as passengers. Returns `None` when fewer than
/// [`MIN_RANKED_MEMBERS`] members can be counted.
pub fn club_balance(members: &[MemberSnapshot]) -> Option<Balance> {
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

/// Scores one club for a trainer with the given daily pace.
///
/// Returns `Ok(None)` when the club is left out: it has fewer than
/// [`MIN_RANKED_MEMBERS`] rankable members, it is closed (unless
/// `include_closed` is set), or balance scoring is on and the club has no
/// balance score.
pub fn score_club<'a>(
    pace: f64,
    club: &'a Club,
    options: &ScoringOptions,
) -> Result<Option<ScoredClub<'a>>, ScoringError> {
    validate(pace, options)?;
    Ok(score_valid(pace, club, options))
}

/// Scores and ranks clubs for a trainer with the given daily pace, best first.
///
/// ```
/// use uma_stable_match_scoring::{score_clubs, Club, MemberSnapshot, ScoringOptions};
///
/// let members = (1..=10)
///     .map(|i| MemberSnapshot {
///         viewer_id: i,
///         daily_fans: vec![100_000_000, 100_000_000 + i as i64 * 1_000_000, 100_000_000 + i as i64 * 2_000_000],
///         ..Default::default()
///     })
///     .collect();
/// let clubs = vec![Club { name: "Example Club".into(), member_count: Some(10), members, ..Default::default() }];
///
/// let options = ScoringOptions { balance_weight: Some(0.5), ..Default::default() };
/// let ranked = score_clubs(7_000_000.0, &clubs, &options).unwrap();
///
/// assert_eq!(ranked[0].club.name, "Example Club");
/// assert_eq!(ranked[0].club_size, 11);
/// ```
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

    // Club size after you join: current members plus you, never above the cap.
    // Brand-new members without enough data are counted but placed below you.
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
mod tests {
    use super::*;

    const M: i64 = 1_000_000;

    /// A member gaining `per_day` fans each day for `days` days.
    fn member(id: u64, per_day: i64, days: usize) -> MemberSnapshot {
        let start = 100 * M;
        MemberSnapshot {
            viewer_id: id,
            year: Some(2026),
            month: Some(10),
            daily_fans: (0..days).map(|d| start + per_day * d as i64).collect(),
        }
    }

    /// A club whose members gain the given daily amounts over 10 days.
    fn club(name: &str, daily_gains: &[i64]) -> Club {
        Club {
            circle_id: 1,
            name: name.into(),
            member_count: Some(daily_gains.len() as u32),
            join_style: Some(JoinStyle::Approval),
            members: daily_gains
                .iter()
                .enumerate()
                .map(|(i, g)| member(i as u64 + 1, *g, 10))
                .collect(),
            ..Default::default()
        }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn place_and_fit_follow_the_formula() {
        // 10 members at 1M..10M per day; you gain 7.5M/day.
        let gains: Vec<i64> = (1..=10).map(|g| g * M).collect();
        let c = club("Test", &gains);
        let s = score_club(7.5e6, &c, &ScoringOptions::default())
            .unwrap()
            .unwrap();

        assert_eq!(s.club_size, 11);
        assert_eq!(s.your_place, 4); // 8M, 9M, 10M are faster
        assert!(approx(s.median_per_day, 5.5e6));
        let expected = (4.0 / 11.0 - 0.4f64).abs() + 0.3 * (5.5e6 / 7.5e6 - 0.85f64).abs();
        assert!(approx(s.fit, expected));
        assert!(approx(s.score, s.fit));
        assert!(s.balance.is_none());
    }

    #[test]
    fn club_size_is_capped_at_thirty() {
        let mut c = club("Full", &[5 * M; 12]);
        c.member_count = Some(30);
        let s = score_club(1e6, &c, &ScoringOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(s.club_size, CLUB_CAP);
        assert_eq!(s.your_place, 13);
    }

    #[test]
    fn missing_member_count_uses_current_members() {
        let mut c = club("Unknown size", &[5 * M; 9]);
        c.member_count = None;
        let s = score_club(6e6, &c, &ScoringOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(s.club_size, 10);
        assert_eq!(s.your_place, 1);
    }

    #[test]
    fn new_members_count_towards_size_but_not_place() {
        let mut c = club("Recruiting", &[5 * M; 8]);
        // Two members who joined on the last day: current, but unrankable.
        for id in [100, 101] {
            let mut fans = vec![0; 10];
            fans[9] = 50 * M;
            c.members.push(MemberSnapshot {
                viewer_id: id,
                year: Some(2026),
                month: Some(10),
                daily_fans: fans,
            });
        }
        c.member_count = Some(10);
        let s = score_club(4e6, &c, &ScoringOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(s.ranked_members, 8);
        assert_eq!(s.club_size, 11);
        assert_eq!(s.your_place, 9);
    }

    #[test]
    fn departed_members_are_ignored() {
        let mut c = club("Turnover", &[5 * M; 8]);
        let mut left = member(99, 50 * M, 10);
        for v in left.daily_fans.iter_mut().skip(5) {
            *v = 0;
        }
        c.members.push(left);
        let s = score_club(6e6, &c, &ScoringOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(s.ranked_members, 8);
        assert_eq!(s.your_place, 1);
    }

    #[test]
    fn small_clubs_are_left_out() {
        let c = club("Tiny", &[5 * M; 7]);
        assert!(score_club(5e6, &c, &ScoringOptions::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn closed_clubs_need_include_closed() {
        let mut c = club("Closed", &[5 * M; 10]);
        c.join_style = Some(JoinStyle::Closed);
        let default = ScoringOptions::default();
        assert!(score_club(5e6, &c, &default).unwrap().is_none());
        let include = ScoringOptions {
            include_closed: true,
            ..Default::default()
        };
        assert!(score_club(5e6, &c, &include).unwrap().is_some());
    }

    #[test]
    fn balance_of_an_even_club() {
        let c = club("Even", &[5 * M; 10]);
        let b = club_balance(&c.members).unwrap();
        assert_eq!(b.score, 100);
        assert_eq!(b.passengers, 0);
        assert_eq!(b.counted, 10);
        assert!(approx(b.gini, 0.0));
        assert!(approx(b.top3_share, 0.3));
        assert!(approx(b.bottom_quarter_per_day, 5e6));
    }

    #[test]
    fn passengers_use_the_higher_threshold() {
        // Median 8M, so the floor is max(1M, 2M) = 2M.
        let mut gains = vec![8 * M; 8];
        gains.extend([1_500_000, 500_000]);
        let b = club_balance(&club("Mixed", &gains).members).unwrap();
        assert_eq!(b.passengers, 2);

        // Median 2M, so the floor is the 1M minimum.
        let mut gains = vec![2 * M; 8];
        gains.extend([900_000, 1_100_000]);
        let b = club_balance(&club("Low", &gains).members).unwrap();
        assert_eq!(b.passengers, 1);
    }

    #[test]
    fn balance_formula() {
        let gains: Vec<i64> = (1..=10).map(|g| g * M).collect();
        let b = club_balance(&club("Spread", &gains).members).unwrap();
        let rates: Vec<f64> = (1..=10).map(|g| g as f64 * 1e6).collect();
        let g = gini(&rates);
        // Floor = max(1M, 0.25 × 5.5M) = 1.375M: only the 1M member is a passenger.
        assert_eq!(b.passengers, 1);
        let expected = (100.0 * (0.5 * (1.0 - g) + 0.5 * (1.0 - 0.1))).round();
        assert_eq!(f64::from(b.score), expected);
        assert!(approx(b.top3_share, 27.0 / 55.0));
        assert!(approx(b.bottom_quarter_per_day, 1.5e6)); // median of 2M, 1M
    }

    #[test]
    fn balance_needs_enough_members() {
        assert!(club_balance(&club("Few", &[5 * M; 7]).members).is_none());
    }

    #[test]
    fn balance_weight_adjusts_score() {
        let mut gains = vec![8 * M; 7];
        gains.extend([0, 0, 0]);
        let c = club("Passengers", &gains);
        let options = ScoringOptions {
            balance_weight: Some(0.8),
            ..Default::default()
        };
        let s = score_club(7e6, &c, &options).unwrap().unwrap();
        let b = s.balance.unwrap();
        assert!(approx(
            s.score,
            s.fit + 0.8 * (1.0 - f64::from(b.score) / 100.0)
        ));
        assert!(s.score > s.fit);
    }

    #[test]
    fn ranking_is_best_first_and_balance_can_reorder() {
        // Same fit, but "Carried" has passengers.
        let even = club("Even", &[6 * M; 10]);
        let mut carried_gains = vec![10 * M; 5];
        carried_gains.extend([2 * M; 5]);
        let carried = club("Carried", &carried_gains);
        let clubs = vec![carried, even];

        let plain = score_clubs(7e6, &clubs, &ScoringOptions::default()).unwrap();
        assert!(plain.windows(2).all(|w| w[0].score <= w[1].score));

        let balanced = score_clubs(
            7e6,
            &clubs,
            &ScoringOptions {
                balance_weight: Some(1.0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(balanced[0].club.name, "Even");
        assert!(balanced.windows(2).all(|w| w[0].score <= w[1].score));
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let clubs = vec![club("Any", &[5 * M; 10])];
        let opts = ScoringOptions::default();
        assert_eq!(
            score_clubs(0.0, &clubs, &opts).unwrap_err(),
            ScoringError::InvalidPace(0.0)
        );
        assert!(score_clubs(f64::NAN, &clubs, &opts).is_err());
        let bad = ScoringOptions {
            balance_weight: Some(1.5),
            ..Default::default()
        };
        assert_eq!(
            score_clubs(5e6, &clubs, &bad).unwrap_err(),
            ScoringError::InvalidBalanceWeight(1.5)
        );
    }
}
