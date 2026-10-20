//! Request parameters.

use serde::Serialize;

/// Which club to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleLookup {
    /// By club ID.
    CircleId(u64),
    /// The club a trainer belongs to.
    ViewerId(u64),
}

/// Parameters for `GET /api/v4/circles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircleRequest {
    /// Which club to fetch.
    pub lookup: CircleLookup,
    /// Year and month (1–12); the API defaults to the current game month.
    pub period: Option<(i32, u32)>,
}

impl CircleRequest {
    /// Fetches a club by ID.
    pub fn by_circle_id(circle_id: u64) -> Self {
        Self {
            lookup: CircleLookup::CircleId(circle_id),
            period: None,
        }
    }

    /// Fetches the club a trainer belongs to.
    pub fn by_viewer_id(viewer_id: u64) -> Self {
        Self {
            lookup: CircleLookup::ViewerId(viewer_id),
            period: None,
        }
    }

    /// Fetches a specific month instead of the current one.
    pub fn for_month(self, year: i32, month: u32) -> Self {
        Self {
            period: Some((year, month)),
            ..self
        }
    }

    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = vec![match self.lookup {
            CircleLookup::CircleId(id) => ("circle_id", id.to_string()),
            CircleLookup::ViewerId(id) => ("viewer_id", id.to_string()),
        }];
        if let Some((year, month)) = self.period {
            query.push(("year", year.to_string()));
            query.push(("month", month.to_string()));
        }
        query
    }
}

/// Sort field for the club list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CircleSort {
    /// Club name.
    Name,
    /// Member count.
    MemberCount,
    /// Monthly rank.
    MonthlyRank,
    /// Monthly points.
    MonthlyPoint,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// Parameters for `GET /api/v4/circles/list`. Unset fields use the API's defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ListCirclesRequest {
    /// Page number (0-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Page size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Partial match on club name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Minimum member count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_members: Option<u32>,
    /// Worst monthly rank to include (lower is better).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rank: Option<u32>,
    /// Sort field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<CircleSort>,
    /// Sort direction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_dir: Option<SortDir>,
    /// General search across club, leader and member IDs and names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl ListCirclesRequest {
    /// The same request for another page.
    pub fn page(&self, page: u32) -> Self {
        Self {
            page: Some(page),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GainsSort {
    #[serde(rename = "gain_3d")]
    Gain3d,
    #[serde(rename = "gain_7d")]
    Gain7d,
    #[serde(rename = "gain_30d")]
    Gain30d,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RankingsGainsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<GainsSort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl RankingsGainsRequest {
    pub fn for_viewer(viewer_id: u64) -> Self {
        Self {
            query: Some(viewer_id.to_string()),
            ..Default::default()
        }
    }

    pub fn page(&self, page: u32) -> Self {
        Self {
            page: Some(page),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests;
