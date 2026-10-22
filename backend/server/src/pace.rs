//! Works out a trainer's pace from their trainer ID.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uma_stable_match_scoring::daily_pace;
use uma_stable_match_umamoe_client::{
    CircleRequest, CircleResponse, Client, Error, RankingsGainsRequest, StatusCode,
};

use crate::details::VISITOR_MAX_PAUSE;
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
}

impl PaceLookup {
    /// Reuses each result for `ttl`.
    pub fn new(client: Client, ttl: Duration) -> Self {
        Self {
            client: client.with_max_pause(VISITOR_MAX_PAUSE),
            ttl,
            entries: Arc::default(),
            shared: SharedCache::none(),
        }
    }

    /// Checks `shared` before uma.moe, and stores each result in it.
    pub fn with_shared(self, shared: SharedCache) -> Self {
        Self { shared, ..self }
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

        let outcome = match period {
            None => {
                self.lookup_club(CircleRequest::by_viewer_id(viewer_id), viewer_id)
                    .await
            }
            Some(period) => self.lookup_last_month(viewer_id, period).await,
        };
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
                tracing::warn!(status = ?error.status(), "trainer pace lookup failed");
                Err(PaceError::Upstream(error))
            }
        }
    }

    async fn lookup_last_month(
        &self,
        viewer_id: u64,
        (year, month): Period,
    ) -> Result<Outcome, PaceError> {
        let request = CircleRequest::by_viewer_id(viewer_id).for_month(year, month);
        let outcome = self.lookup_club(request, viewer_id).await?;
        if matches!(outcome, Outcome::Pace(_)) {
            return Ok(outcome);
        }
        let gains = match self
            .client
            .rankings_gains(&RankingsGainsRequest::for_viewer(viewer_id))
            .await
        {
            Ok(gains) => gains,
            Err(error) if error.status() == Some(StatusCode::NOT_FOUND) => return Ok(outcome),
            Err(error) => {
                tracing::warn!(status = ?error.status(), "trainer gains lookup failed");
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

fn pace_from_response(response: &CircleResponse, viewer_id: u64) -> Outcome {
    let snapshot = response
        .members
        .iter()
        .filter(|m| m.viewer_id == viewer_id)
        .max_by_key(|m| (m.year, m.month));
    match snapshot {
        None => Outcome::NotFound,
        Some(member) => match daily_pace(&member.daily_fans) {
            Some(pace) if pace > 0.0 => Outcome::Pace(pace),
            _ => Outcome::NotEnoughData,
        },
    }
}

#[cfg(test)]
mod tests;
