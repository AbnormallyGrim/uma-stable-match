//! Request parameters.

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

#[cfg(test)]
mod tests;
