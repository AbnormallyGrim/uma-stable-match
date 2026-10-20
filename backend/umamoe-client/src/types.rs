//! API response types and their conversion into scoring types.

use serde::{Deserialize, Serialize};
use uma_stable_match_scoring::{Club, JoinStyle, MemberSnapshot};

/// A club as returned by the API. Missing fields take their default.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Circle {
    /// Club ID.
    pub circle_id: u64,
    /// Club name.
    pub name: String,
    /// Club description.
    pub comment: Option<String>,
    /// Leader's trainer ID.
    pub leader_viewer_id: Option<u64>,
    /// Leader's trainer name.
    pub leader_name: Option<String>,
    /// Official member count.
    pub member_count: Option<i32>,
    /// Join style code (1 = open, 2 = approval, 3 = closed).
    pub join_style: Option<i32>,
    /// Playstyle code.
    pub policy: Option<i32>,
    /// Creation time (ISO 8601).
    pub created_at: Option<String>,
    /// Last update time (ISO 8601).
    pub last_updated: Option<String>,
    /// This month's display rank.
    pub monthly_rank: Option<i32>,
    /// This month's display points.
    pub monthly_point: Option<i64>,
    /// Last month's rank.
    pub last_month_rank: Option<i32>,
    /// Last month's points.
    pub last_month_point: Option<i64>,
    /// Whether the club is archived.
    pub archived: Option<bool>,
    /// Time of yesterday's snapshot (ISO 8601).
    pub yesterday_updated: Option<String>,
    /// Yesterday's points.
    pub yesterday_points: Option<i64>,
    /// Yesterday's rank.
    pub yesterday_rank: Option<i32>,
    /// Live fan count, when in the live refresh window.
    pub live_points: Option<i64>,
    /// Live rank, when in the live refresh window.
    pub live_rank: Option<i32>,
    /// Time of the last live update (ISO 8601).
    pub last_live_update: Option<String>,
}

/// One member's fan data for a month.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CircleMember {
    /// Row ID.
    pub id: i64,
    /// Club ID.
    pub circle_id: u64,
    /// Trainer ID.
    pub viewer_id: u64,
    /// Trainer name.
    pub trainer_name: Option<String>,
    /// uma.moe suspicious-activity score.
    pub shame_score: Option<i32>,
    /// Year.
    pub year: Option<i32>,
    /// Month (1–12).
    pub month: Option<i32>,
    /// Cumulative fans for each day of the month.
    pub daily_fans: Vec<i64>,
    /// Last update time (ISO 8601).
    pub last_updated: Option<String>,
    /// Previous club ID.
    pub previous_circle_id: Option<u64>,
    /// Previous club name.
    pub previous_circle_name: Option<String>,
    /// Fan count at the start of next month.
    pub next_month_start: Option<i64>,
}

/// Response from `GET /api/v4/circles`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CircleResponse {
    /// The club.
    pub circle: Circle,
    /// Member fan data for the month.
    pub members: Vec<CircleMember>,
    /// Tier code, 1 (D) to 11 (SS).
    pub club_rank: Option<i32>,
    /// Fans needed to reach the next tier.
    pub fans_to_next_tier: Option<i64>,
    /// Fans above the tier below.
    pub fans_to_lower_tier: Option<i64>,
    /// Yesterday's fans needed to reach the next tier.
    pub yesterday_fans_to_next_tier: Option<i64>,
    /// Yesterday's fans above the tier below.
    pub yesterday_fans_to_lower_tier: Option<i64>,
}

/// Response from `GET /api/v4/circles/list`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CircleListResponse {
    /// Clubs on this page.
    pub circles: Vec<Circle>,
    /// Total matching clubs.
    pub total: i64,
    /// Page number (0-indexed).
    pub page: i64,
    /// Page size.
    pub limit: i64,
    /// Total pages.
    pub total_pages: i64,
}

impl CircleListResponse {
    /// Whether a later page exists.
    pub fn has_next_page(&self) -> bool {
        self.page + 1 < self.total_pages
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GainsRanking {
    pub viewer_id: u64,
    pub trainer_name: Option<String>,
    pub gain_3d: Option<i64>,
    pub gain_7d: Option<i64>,
    pub gain_30d: Option<i64>,
    pub rank_3d: Option<i32>,
    pub rank_7d: Option<i32>,
    pub rank_30d: Option<i32>,
    pub circle_id: Option<u64>,
    pub circle_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GainsRankingsResponse {
    pub rankings: Vec<GainsRanking>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
    pub sort_by: Option<String>,
}

impl GainsRankingsResponse {
    pub fn has_next_page(&self) -> bool {
        self.page + 1 < self.total_pages
    }

    pub fn viewer(&self, viewer_id: u64) -> Option<&GainsRanking> {
        self.rankings.iter().find(|r| r.viewer_id == viewer_id)
    }
}

impl From<Circle> for Club {
    /// Club details only: no members or tier.
    fn from(c: Circle) -> Self {
        let mut club = Club::new(c.circle_id, c.name);
        club.member_count = to_u32(c.member_count);
        club.monthly_rank = to_u32(c.monthly_rank);
        club.last_month_rank = to_u32(c.last_month_rank);
        club.join_style = c
            .join_style
            .and_then(|code| u8::try_from(code).ok())
            .and_then(JoinStyle::from_code);
        club.policy = to_u32(c.policy);
        club.comment = c.comment;
        club
    }
}

impl From<CircleResponse> for Club {
    fn from(r: CircleResponse) -> Self {
        let mut club = Club::from(r.circle);
        club.club_rank = r.club_rank.and_then(|code| u8::try_from(code).ok());
        club.members = r.members.into_iter().map(MemberSnapshot::from).collect();
        club
    }
}

impl From<CircleMember> for MemberSnapshot {
    fn from(m: CircleMember) -> Self {
        let mut member = MemberSnapshot::new(m.viewer_id, m.daily_fans);
        member.year = m.year;
        member.month = to_u32(m.month);
        member
    }
}

fn to_u32(value: Option<i32>) -> Option<u32> {
    value.and_then(|v| u32::try_from(v).ok())
}

#[cfg(test)]
mod tests;
