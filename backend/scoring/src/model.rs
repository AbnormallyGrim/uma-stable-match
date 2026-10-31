//! Input data: clubs and member snapshots.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Maximum members per club.
pub const CLUB_CAP: u32 = 30;

/// One member's cumulative fan count for each day of a month.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct MemberSnapshot {
    /// Trainer ID.
    pub viewer_id: u64,
    /// Snapshot year.
    pub year: Option<i32>,
    /// Snapshot month (1–12).
    pub month: Option<u32>,
    /// Cumulative fans per day: 0 = no data, negative = previous club.
    pub daily_fans: Vec<i64>,
    /// Pace to blend with this month's, such as last month's; see
    /// [`ScoringOptions::prior_k`](crate::ScoringOptions::prior_k).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub prior: Option<f64>,
    /// Trainer name, for display. Not used in scoring.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub name: Option<String>,
}

impl MemberSnapshot {
    /// A snapshot with no year or month set.
    pub fn new(viewer_id: u64, daily_fans: Vec<i64>) -> Self {
        Self {
            viewer_id,
            daily_fans,
            ..Self::default()
        }
    }
}

/// How a club accepts new members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum JoinStyle {
    /// Anyone can join.
    Open,
    /// Applications need approval.
    Approval,
    /// Not accepting members.
    Closed,
}

impl JoinStyle {
    /// Converts the game's join style code (1 = open, 2 = approval, 3 = closed).
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Open),
            2 => Some(Self::Approval),
            3 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// A club and its member snapshots for one month.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Club {
    /// Club ID.
    pub circle_id: u64,
    /// Club name.
    pub name: String,
    /// Official member count.
    pub member_count: Option<u32>,
    /// This month's rank.
    pub monthly_rank: Option<u32>,
    /// Last month's rank.
    pub last_month_rank: Option<u32>,
    /// How the club accepts members.
    pub join_style: Option<JoinStyle>,
    /// Playstyle code.
    pub policy: Option<u32>,
    /// Tier code, 1 (D) to 11 (SS).
    pub club_rank: Option<u8>,
    /// Club description.
    pub comment: Option<String>,
    /// Member snapshots.
    pub members: Vec<MemberSnapshot>,
}

impl Club {
    /// A club with only its ID and name set.
    pub fn new(circle_id: u64, name: impl Into<String>) -> Self {
        Self {
            circle_id,
            name: name.into(),
            ..Self::default()
        }
    }

    /// Open spots, if the member count is known.
    pub fn open_spots(&self) -> Option<u32> {
        self.member_count.map(|n| CLUB_CAP.saturating_sub(n))
    }

    /// Tier label such as `"A+"`, if known.
    pub fn tier(&self) -> Option<&'static str> {
        self.club_rank.and_then(tier_label)
    }
}

const TIER_LABELS: [&str; 11] = ["D", "D+", "C", "C+", "B", "B+", "A", "A+", "S", "S+", "SS"];

/// Converts a tier code (1–11) to its label, `"D"` to `"SS"`.
pub fn tier_label(code: u8) -> Option<&'static str> {
    (1..=11)
        .contains(&code)
        .then(|| TIER_LABELS[usize::from(code) - 1])
}

#[cfg(test)]
mod tests;
