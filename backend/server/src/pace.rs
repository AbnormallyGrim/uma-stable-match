//! Works out a trainer's pace from their trainer ID.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uma_stable_match_scoring::daily_pace;
use uma_stable_match_umamoe_client::{
    CircleRequest, CircleResponse, Client, Error, Priority, RankingsGainsRequest,
    RankingsMonthlyRequest, StatusCode,
};

use crate::details::{ClubDetails, VISITOR_MAX_PAUSE};
use crate::pool::Period;
use crate::shared::SharedCache;

/// How long a looked-up pace is reused.
pub const DEFAULT_PACE_TTL: Duration = Duration::from_secs(10 * 60);

/// Why a trainer's pace couldn't be found. Messages never include the trainer ID.
#[derive(Debug, thiserror::Error)]
pub enum PaceError {
    /// uma.moe doesn't know the trainer, or they aren't in the club it returned.
    #[error("trainer not found")]
    NotFound,
    /// Fewer than 3 days of data, or no gain this month.
    #[error("not enough fan data this month to work out a pace")]
    NotEnoughData,
    /// The uma.moe request failed.
    #[error("could not reach uma.moe")]
    Upstream(#[source] Error),
}

type Entries = HashMap<(u64, Option<Period>), (Instant, Outcome)>;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Outcome {
    Pace(f64),
    NotFound,
    NotEnoughData,
}

impl Outcome {
    fn into_result(self) -> Result<f64, PaceError> {
        match self {
            Self::Pace(pace) => Ok(pace),
            Self::NotFound => Err(PaceError::NotFound),
            Self::NotEnoughData => Err(PaceError::NotEnoughData),
        }
    }
}

/// Looks up trainers' paces, reusing results for a while. Clones share the cache.
#[derive(Debug, Clone)]
pub struct PaceLookup {
    client: Client,
    ttl: Duration,
    entries: Arc<Mutex<Entries>>,
    shared: SharedCache,
    details: Option<ClubDetails>,
}

impl PaceLookup {
    /// Reuses each result for `ttl`.
    pub fn new(client: Client, ttl: Duration) -> Self {
        Self {
            client: client.with_max_pause(VISITOR_MAX_PAUSE),
            ttl,
            entries: Arc::default(),
            shared: SharedCache::none(),
            details: None,
        }
    }

    /// Checks `shared` before uma.moe, and stores each result in it.
    pub fn with_shared(self, shared: SharedCache) -> Self {
        Self { shared, ..self }
    }

    /// Reads trainers' clubs through `details`, so cached clubs don't need another uma.moe call.
    pub fn with_details(self, details: ClubDetails) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }

    /// The trainer's daily fan gain this month, or in `period` if given. For a given period it
    /// falls back to the 7-day gain ÷ 7 when that month has too little data.
    ///
    /// Found and not-found results are cached per month; uma.moe failures aren't.
    pub async fn pace(&self, viewer_id: u64, period: Option<Period>) -> Result<f64, PaceError> {
        if let Some(outcome) = self.cached(viewer_id, period) {
            count_pace("memory");
            return outcome.into_result();
        }
        let key = self.shared.pace_key(viewer_id, period);
        if let Some(outcome) = self.shared.get::<Outcome>(&key).await {
            count_pace("redis");
            self.store(viewer_id, period, outcome);
            return outcome.into_result();
        }

        let outcome = self.lookup(viewer_id, period).await;
        count_pace(if outcome.is_ok() { "umamoe" } else { "error" });
        let outcome = outcome?;
        self.store(viewer_id, period, outcome);
        self.shared.set(&key, &outcome, self.ttl).await;
        outcome.into_result()
    }

    /// Whether a fresh result for this trainer is cached.
    pub fn is_cached(&self, viewer_id: u64, period: Option<Period>) -> bool {
        self.cached(viewer_id, period).is_some()
    }

    async fn lookup_club(
        &self,
        request: CircleRequest,
        viewer_id: u64,
    ) -> Result<Outcome, PaceError> {
        match self.client.circle(&request).await {
            Ok(response) => Ok(pace_from_response(&response, viewer_id)),
            Err(error) if error.status() == Some(StatusCode::NOT_FOUND) => Ok(Outcome::NotFound),
            Err(error) => {
                tracing::warn!(
                    trainer_id = viewer_id,
                    month = month_label(request.period),
                    origin = error.origin(),
                    status = error.status().map(|status| status.as_u16()),
                    %error,
                    "trainer pace lookup failed"
                );
                Err(PaceError::Upstream(error))
            }
        }
    }

    async fn lookup(&self, viewer_id: u64, period: Option<Period>) -> Result<Outcome, PaceError> {
        let ranked = match self.ranked_club(viewer_id, period).await {
            Some(circle_id) => self.pace_in_club(circle_id, viewer_id, period).await,
            None => Outcome::NotFound,
        };
        let outcome = if ranked == Outcome::NotFound {
            let mut request = CircleRequest::by_viewer_id(viewer_id);
            if let Some((year, month)) = period {
                request = request.for_month(year, month);
            }
            self.lookup_club(request, viewer_id).await?
        } else {
            ranked
        };
        match period {
            Some(_) if !matches!(outcome, Outcome::Pace(_)) => {
                self.seven_day_gains(viewer_id, outcome).await
            }
            _ => Ok(outcome),
        }
    }

    async fn ranked_club(&self, viewer_id: u64, period: Option<Period>) -> Option<u64> {
        let mut request = RankingsMonthlyRequest::for_viewer(viewer_id);
        if let Some((year, month)) = period {
            request = request.for_month(year, month);
        }
        match self.client.rankings_monthly(&request).await {
            Ok(rankings) => rankings.viewer(viewer_id).and_then(|row| row.circle_id),
            Err(error) if error.status() == Some(StatusCode::NOT_FOUND) => None,
            Err(error) => {
                tracing::warn!(
                    trainer_id = viewer_id,
                    month = month_label(period),
                    origin = error.origin(),
                    status = error.status().map(|status| status.as_u16()),
                    %error,
                    "trainer club lookup failed"
                );
                None
            }
        }
    }

    async fn pace_in_club(
        &self,
        circle_id: u64,
        viewer_id: u64,
        period: Option<Period>,
    ) -> Outcome {
        let result = match &self.details {
            Some(details) if details.period() == period => details
                .get(circle_id, Priority::Interactive)
                .await
                .map(|club| {
                    pace_of(
                        club.members.iter().map(|m| {
                            (
                                m.viewer_id,
                                m.year,
                                m.month.and_then(|n| i32::try_from(n).ok()),
                                m.daily_fans.as_slice(),
                            )
                        }),
                        viewer_id,
                    )
                })
                .map_err(|error| (error.origin(), error.to_string())),
            _ => {
                let mut request = CircleRequest::by_circle_id(circle_id);
                if let Some((year, month)) = period {
                    request = request.for_month(year, month);
                }
                self.client
                    .circle(&request)
                    .await
                    .map(|response| pace_from_response(&response, viewer_id))
                    .map_err(|error| (error.origin(), error.to_string()))
            }
        };
        result.unwrap_or_else(|(origin, error)| {
            tracing::warn!(
                trainer_id = viewer_id,
                circle_id,
                month = month_label(period),
                origin,
                error,
                "trainer club details lookup failed"
            );
            Outcome::NotFound
        })
    }

    async fn seven_day_gains(
        &self,
        viewer_id: u64,
        outcome: Outcome,
    ) -> Result<Outcome, PaceError> {
        let gains = match self
            .client
            .rankings_gains(&RankingsGainsRequest::for_viewer(viewer_id))
            .await
        {
            Ok(gains) => gains,
            Err(error) if error.status() == Some(StatusCode::NOT_FOUND) => return Ok(outcome),
            Err(error) => {
                tracing::warn!(
                    trainer_id = viewer_id,
                    origin = error.origin(),
                    status = error.status().map(|status| status.as_u16()),
                    %error,
                    "trainer gains lookup failed"
                );
                return Err(PaceError::Upstream(error));
            }
        };
        Ok(match gains.viewer(viewer_id).and_then(|r| r.gain_7d) {
            Some(gain) if gain > 0 => Outcome::Pace(gain as f64 / 7.0),
            _ => outcome,
        })
    }

    fn cached(&self, viewer_id: u64, period: Option<Period>) -> Option<Outcome> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries
            .get(&(viewer_id, period))
            .filter(|(at, _)| at.elapsed() < self.ttl)
            .map(|(_, outcome)| *outcome)
    }

    fn store(&self, viewer_id: u64, period: Option<Period>, outcome: Outcome) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|_, (at, _)| at.elapsed() < self.ttl);
        entries.insert((viewer_id, period), (Instant::now(), outcome));
    }
}

fn count_pace(source: &'static str) {
    metrics::counter!("pace_lookups_total", "source" => source).increment(1);
}

/// `period` as `YYYY-MM` for logs, or `None` for the current month.
fn month_label(period: Option<Period>) -> Option<String> {
    period.map(|(year, month)| format!("{year}-{month:02}"))
}

fn pace_from_response(response: &CircleResponse, viewer_id: u64) -> Outcome {
    pace_of(
        response
            .members
            .iter()
            .map(|m| (m.viewer_id, m.year, m.month, m.daily_fans.as_slice())),
        viewer_id,
    )
}

fn pace_of<'a>(
    members: impl Iterator<Item = (u64, Option<i32>, Option<i32>, &'a [i64])>,
    viewer_id: u64,
) -> Outcome {
    let snapshot = members
        .filter(|(id, ..)| *id == viewer_id)
        .max_by_key(|(_, year, month, _)| (*year, *month));
    match snapshot {
        None => Outcome::NotFound,
        Some((.., daily_fans)) => match daily_pace(daily_fans) {
            Some(pace) if pace > 0.0 => Outcome::Pace(pace),
            _ => Outcome::NotEnoughData,
        },
    }
}

#[cfg(test)]
mod tests;
