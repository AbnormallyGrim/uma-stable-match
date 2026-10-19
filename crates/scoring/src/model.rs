//! Input data: clubs, their members, and club metadata.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Maximum number of members a club can have.
pub const CLUB_CAP: u32 = 30;

/// One member's fan history for a single month.
///
/// `daily_fans` holds the member's **cumulative** fan count for each day of the
/// month, indexed from day 1. A value of `0` means no data for that day (for
/// example, before the member joined or after they left). A **negative** value
/// is a fan count recorded while the member was in a previous club.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MemberSnapshot {
    /// The member's trainer ID.
    pub viewer_id: u64,
    /// Year of the snapshot, if known.
    pub year: Option<i32>,
    /// Month of the snapshot (1–12), if known.
    pub month: Option<u32>,
    /// Cumulative fan count per day of the month.
    pub daily_fans: Vec<i64>,
}

/// How a club accepts new members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum JoinStyle {
    /// Anyone can join immediately.
    Open,
    /// The leader or an officer must approve applications.
    Approval,
    /// Not accepting new members.
    Closed,
}

impl JoinStyle {
    /// Converts the numeric join style used by the game data (1 = open,
    /// 2 = approval, 3 = closed).
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Open),
            2 => Some(Self::Approval),
            3 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// A club and its members for the month being scored.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Club {
    /// The club's ID.
    pub circle_id: u64,
    /// The club's name.
    pub name: String,
    /// Official member count, if known. Used to work out the club's size after
    /// you join.
    pub member_count: Option<u32>,
    /// This month's rank.
    pub monthly_rank: Option<u32>,
    /// Last month's rank.
    pub last_month_rank: Option<u32>,
    /// How the club accepts members.
    pub join_style: Option<JoinStyle>,
    /// Playstyle code, as used by the game data.
    pub policy: Option<u32>,
    /// Club tier code, from 1 (D) to 11 (SS). See [`tier_label`].
    pub club_rank: Option<u8>,
    /// The club's description.
    pub comment: Option<String>,
    /// Member snapshots for the month.
    pub members: Vec<MemberSnapshot>,
}

impl Club {
    /// Number of open spots, if the member count is known.
    pub fn open_spots(&self) -> Option<u32> {
        self.member_count.map(|n| CLUB_CAP.saturating_sub(n))
    }

    /// The club's tier label, such as `"A+"`, if known.
    pub fn tier(&self) -> Option<&'static str> {
        self.club_rank.and_then(tier_label)
    }
}

const TIER_LABELS: [&str; 11] = ["D", "D+", "C", "C+", "B", "B+", "A", "A+", "S", "S+", "SS"];

/// Converts a club tier code (1–11) to its label, from `"D"` to `"SS"`.
///
/// ```
/// use uma_stable_match_scoring::tier_label;
///
/// assert_eq!(tier_label(8), Some("A+"));
/// assert_eq!(tier_label(12), None);
/// ```
pub fn tier_label(code: u8) -> Option<&'static str> {
    (1..=11)
        .contains(&code)
        .then(|| TIER_LABELS[usize::from(code) - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_labels_cover_every_code() {
        assert_eq!(tier_label(0), None);
        assert_eq!(tier_label(1), Some("D"));
        assert_eq!(tier_label(6), Some("B+"));
        assert_eq!(tier_label(11), Some("SS"));
        assert_eq!(tier_label(12), None);
    }

    #[test]
    fn join_style_codes() {
        assert_eq!(JoinStyle::from_code(1), Some(JoinStyle::Open));
        assert_eq!(JoinStyle::from_code(2), Some(JoinStyle::Approval));
        assert_eq!(JoinStyle::from_code(3), Some(JoinStyle::Closed));
        assert_eq!(JoinStyle::from_code(0), None);
    }

    #[test]
    fn open_spots_never_negative() {
        let mut club = Club {
            member_count: Some(27),
            ..Default::default()
        };
        assert_eq!(club.open_spots(), Some(3));
        club.member_count = Some(31);
        assert_eq!(club.open_spots(), Some(0));
        club.member_count = None;
        assert_eq!(club.open_spots(), None);
    }
}
