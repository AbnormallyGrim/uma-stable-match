use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use uma_stable_match_scoring::{JoinStyle, CLUB_CAP, MIN_RANKED_MEMBERS};
use uma_stable_match_umamoe_client::Circle;

use crate::calibration::Calibration;

pub const TARGET_MEDIAN_RATIO: f64 = 0.85;

pub const DEFAULT_SHORTLIST_SIZE: usize = 120;

const JST_OFFSET_SECONDS: i32 = 9 * 3600;
const SECONDS_PER_DAY: f64 = 86_400.0;
const MIN_ELAPSED_DAYS: f64 = 1.0;

pub fn jst() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET_SECONDS).expect("valid offset")
}

pub fn game_month_start(at: DateTime<Utc>) -> DateTime<Utc> {
    let local = at.with_timezone(&jst());
    let (mut year, mut month) = (local.year(), local.month());
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

pub fn elapsed_days(at: DateTime<Utc>) -> f64 {
    (at - game_month_start(at)).num_seconds() as f64 / SECONDS_PER_DAY
}

pub fn raw_rate(monthly_point: i64, member_count: u32, at: DateTime<Utc>) -> Option<f64> {
    let days = elapsed_days(at);
    if monthly_point <= 0 || member_count == 0 || days < MIN_ELAPSED_DAYS {
        return None;
    }
    Some(monthly_point as f64 / days / f64::from(member_count))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListedClub {
    pub circle_id: u64,
    pub member_count: u32,
    pub join_style: Option<JoinStyle>,
    pub raw_rate: f64,
}

impl ListedClub {
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
    ln_raw: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ClubPool {
    entries: Vec<Entry>,
    calibration: Calibration,
}

impl ClubPool {
    pub fn new(clubs: impl IntoIterator<Item = ListedClub>) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<Entry> = clubs
            .into_iter()
            .filter(|c| c.raw_rate.is_finite() && c.raw_rate > 0.0)
            .filter(|c| seen.insert(c.circle_id))
            .map(|club| Entry {
                ln_raw: club.raw_rate.ln(),
                club,
            })
            .collect();
        entries.sort_by(|a, b| {
            a.ln_raw
                .total_cmp(&b.ln_raw)
                .then(a.club.circle_id.cmp(&b.club.circle_id))
        });
        Self {
            entries,
            calibration: Calibration::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clubs(&self) -> impl ExactSizeIterator<Item = &ListedClub> {
        self.entries.iter().map(|e| &e.club)
    }

    pub fn calibration(&self) -> Calibration {
        self.calibration
    }

    pub fn set_calibration(&mut self, calibration: Calibration) {
        self.calibration = calibration;
    }

    pub fn estimated_median(&self, club: &ListedClub) -> f64 {
        self.calibration.factor() * club.raw_rate
    }

    pub fn shortlist(&self, target: f64, k: usize, include_closed: bool) -> Vec<u64> {
        if k == 0 || !(target.is_finite() && target > 0.0) {
            return Vec::new();
        }
        let ln_target = target.ln() - self.calibration.factor().ln();
        let eligible = |e: &Entry| include_closed || e.club.join_style != Some(JoinStyle::Closed);

        let split = self.entries.partition_point(|e| e.ln_raw < ln_target);
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
    (entry.ln_raw - ln_target).abs()
}

#[cfg(test)]
mod tests;
