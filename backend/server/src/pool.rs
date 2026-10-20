//! Stage 1: pace estimates from list rows, and nearest-pace shortlists.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use uma_stable_match_scoring::{JoinStyle, CLUB_CAP, MIN_RANKED_MEMBERS};
use uma_stable_match_umamoe_client::Circle;

use crate::calibration::Calibration;

/// Ideal club median as a share of your pace, as in the fit formula.
pub const TARGET_MEDIAN_RATIO: f64 = 0.85;

/// Clubs considered per request by default.
pub const DEFAULT_SHORTLIST_SIZE: usize = 120;

const JST_OFFSET_SECONDS: i32 = 9 * 3600;
const SECONDS_PER_DAY: f64 = 86_400.0;
const MIN_ELAPSED_DAYS: f64 = 1.0;

/// Japan Standard Time (UTC+09:00), which uma.moe uses for months.
pub fn jst() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET_SECONDS).expect("valid offset")
}

/// Start of the game month containing `at`: the 2nd at 00:00 JST.
pub fn game_month_start(at: DateTime<Utc>) -> DateTime<Utc> {
    let local = at.with_timezone(&jst());
    let (mut year, mut month) = (local.year(), local.month());
    // The 1st still belongs to the previous game month.
    if local.day() < 2 {
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
    }
    jst()
        .with_ymd_and_hms(year, month, 2, 0, 0, 0)
        .single()
        .expect("the 2nd of every month exists")
        .with_timezone(&Utc)
}

/// Days from the start of the game month to `at`.
pub fn elapsed_days(at: DateTime<Utc>) -> f64 {
    (at - game_month_start(at)).num_seconds() as f64 / SECONDS_PER_DAY
}

/// Mean daily fans per member this game month, or `None` without points, members or a full day.
pub fn raw_rate(monthly_point: i64, member_count: u32, at: DateTime<Utc>) -> Option<f64> {
    let days = elapsed_days(at);
    if monthly_point <= 0 || member_count == 0 || days < MIN_ELAPSED_DAYS {
        return None;
    }
    Some(monthly_point as f64 / days / f64::from(member_count))
}

/// A club from the list endpoint with its stage-1 estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct ListedClub {
    /// Club ID.
    pub circle_id: u64,
    /// Official member count.
    pub member_count: u32,
    /// Join style, kept so closed clubs can be filtered per request.
    pub join_style: Option<JoinStyle>,
    /// Mean daily fans per member, from [`raw_rate`].
    pub raw_rate: f64,
}

impl ListedClub {
    /// Builds a pool entry, or `None` if the club is archived, full, too small to score or
    /// missing data.
    pub fn from_circle(circle: &Circle) -> Option<Self> {
        if circle.archived == Some(true) {
            return None;
        }
        let member_count = circle
            .member_count
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| (MIN_RANKED_MEMBERS as u32..CLUB_CAP).contains(n))?;
        let at = DateTime::parse_from_rfc3339(circle.last_updated.as_deref()?)
            .ok()?
            .with_timezone(&Utc);
        let raw_rate = raw_rate(circle.monthly_point?, member_count, at)?;
        let join_style = circle
            .join_style
            .and_then(|code| u8::try_from(code).ok())
            .and_then(JoinStyle::from_code);
        Some(Self {
            circle_id: circle.circle_id,
            member_count,
            join_style,
            raw_rate,
        })
    }
}

#[derive(Debug, Clone)]
struct Entry {
    club: ListedClub,
    exact_median: Option<f64>,
    ln_estimate: f64,
}

/// Eligible clubs sorted by estimated median pace.
#[derive(Debug, Clone, Default)]
pub struct ClubPool {
    entries: Vec<Entry>,
    index: HashMap<u64, usize>,
    calibration: Calibration,
}

impl ClubPool {
    /// Builds a pool with no exact medians and no calibration. Later duplicates and invalid rates
    /// are dropped.
    pub fn new(clubs: impl IntoIterator<Item = ListedClub>) -> Self {
        let mut seen = HashSet::new();
        let entries = clubs
            .into_iter()
            .filter(|c| c.raw_rate.is_finite() && c.raw_rate > 0.0)
            .filter(|c| seen.insert(c.circle_id))
            .map(|club| Entry {
                ln_estimate: club.raw_rate.ln(),
                exact_median: None,
                club,
            })
            .collect();
        let mut pool = Self {
            entries,
            index: HashMap::new(),
            calibration: Calibration::default(),
        };
        pool.refresh();
        pool
    }

    /// Number of clubs.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the pool has no clubs.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clubs from the slowest estimate to the fastest.
    pub fn clubs(&self) -> impl ExactSizeIterator<Item = &ListedClub> {
        self.entries.iter().map(|e| &e.club)
    }

    /// The current calibration.
    pub fn calibration(&self) -> Calibration {
        self.calibration
    }

    /// Whether the club is in the pool.
    pub fn contains(&self, circle_id: u64) -> bool {
        self.index.contains_key(&circle_id)
    }

    /// Clubs whose exact median is known.
    pub fn exact_medians(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.exact_median.is_some())
            .count()
    }

    /// A club's median daily pace: exact if known, otherwise calibrated from its raw rate.
    pub fn estimated_median(&self, circle_id: u64) -> Option<f64> {
        let entry = &self.entries[*self.index.get(&circle_id)?];
        Some(
            entry
                .exact_median
                .unwrap_or(self.calibration.factor() * entry.club.raw_rate),
        )
    }

    /// Records a club's exact median from its loaded details. Returns `false` for an unknown club
    /// or an invalid median.
    pub fn set_exact_median(&mut self, circle_id: u64, median: f64) -> bool {
        self.set_exact_medians([(circle_id, median)]) == 1
    }

    /// Records several exact medians, then recalibrates and re-sorts once. Returns how many were
    /// set.
    pub fn set_exact_medians(&mut self, medians: impl IntoIterator<Item = (u64, f64)>) -> usize {
        let mut set = 0;
        for (circle_id, median) in medians {
            if !(median.is_finite() && median > 0.0) {
                continue;
            }
            if let Some(&i) = self.index.get(&circle_id) {
                self.entries[i].exact_median = Some(median);
                set += 1;
            }
        }
        if set > 0 {
            self.calibration = Calibration::from_ratios(
                self.entries
                    .iter()
                    .filter_map(|e| e.exact_median.map(|m| m / e.club.raw_rate)),
            );
            self.refresh();
        }
        set
    }

    fn refresh(&mut self) {
        let ln_factor = self.calibration.factor().ln();
        for entry in &mut self.entries {
            entry.ln_estimate = match entry.exact_median {
                Some(median) => median.ln(),
                None => entry.club.raw_rate.ln() + ln_factor,
            };
        }
        self.entries.sort_by(|a, b| {
            a.ln_estimate
                .total_cmp(&b.ln_estimate)
                .then(a.club.circle_id.cmp(&b.club.circle_id))
        });
        self.index = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.club.circle_id, i))
            .collect();
    }

    /// IDs of the `k` clubs whose estimated median is nearest `target` in log space, nearest
    /// first. Ties go to the lower ID.
    pub fn shortlist(&self, target: f64, k: usize, include_closed: bool) -> Vec<u64> {
        if k == 0 || !(target.is_finite() && target > 0.0) {
            return Vec::new();
        }
        let ln_target = target.ln();
        let eligible = |e: &Entry| include_closed || e.club.join_style != Some(JoinStyle::Closed);

        let split = self.entries.partition_point(|e| e.ln_estimate < ln_target);
        let (mut left, mut right) = (split, split);
        let mut below = self.previous(&mut left, &eligible);
        let mut above = self.next(&mut right, &eligible);

        let mut picked: Vec<(f64, u64)> = Vec::with_capacity(k);
        let mut cutoff = None;
        loop {
            let from_below = match (below, above) {
                (None, None) => break,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (Some(b), Some(a)) => distance(b, ln_target) <= distance(a, ln_target),
            };
            let entry = if from_below { below } else { above }.expect("checked above");
            let d = distance(entry, ln_target);
            if cutoff.is_some_and(|c| d > c) {
                break;
            }
            picked.push((d, entry.club.circle_id));
            // Keep collecting clubs tied with the k-th, so the cut below matches a full sort.
            if picked.len() == k {
                cutoff = Some(d);
            }
            if from_below {
                below = self.previous(&mut left, &eligible);
            } else {
                above = self.next(&mut right, &eligible);
            }
        }

        picked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        picked.truncate(k);
        picked.into_iter().map(|(_, id)| id).collect()
    }

    fn previous(&self, index: &mut usize, eligible: &impl Fn(&Entry) -> bool) -> Option<&Entry> {
        while *index > 0 {
            *index -= 1;
            let entry = &self.entries[*index];
            if eligible(entry) {
                return Some(entry);
            }
        }
        None
    }

    fn next(&self, index: &mut usize, eligible: &impl Fn(&Entry) -> bool) -> Option<&Entry> {
        while let Some(entry) = self.entries.get(*index) {
            *index += 1;
            if eligible(entry) {
                return Some(entry);
            }
        }
        None
    }
}

fn distance(entry: &Entry, ln_target: f64) -> f64 {
    (entry.ln_estimate - ln_target).abs()
}

#[cfg(test)]
mod tests;
