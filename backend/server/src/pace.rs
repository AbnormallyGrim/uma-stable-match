//! Works out a trainer's pace from their trainer ID.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::field::Empty;
use tracing::{Instrument, Span};
use uma_stable_match_scoring::{blended_pace, daily_pace};
use uma_stable_match_umamoe_client::{
    CircleRequest, CircleResponse, Priority, RankingsGainsRequest, RankingsMonthlyRequest,
};

use crate::details::ClubDetails;
use crate::pool::Period;
use crate::priors::month_before;
use crate::shared::SharedCache;
use crate::source::{Source, SourceError};
use crate::traces::in_trace;

/// How long a looked-up pace is reused.
pub const DEFAULT_PACE_TTL: Duration = Duration::from_secs(10 * 60);

/// Why a trainer's pace couldn't be found. Messages never include the trainer ID.
#[derive(Debug, thiserror::Error)]
pub enum PaceError {
    /// The source doesn't know the trainer, or they aren't in the club it returned.
    #[error("trainer not found")]
    NotFound,
    /// Fewer than 3 days of data, or no gain this month.
    #[error("not enough fan data this month to work out a pace")]
    NotEnoughData,
    /// The source request failed. The message names uma.moe for its errors, and is the source's own
    /// public message otherwise.
    #[error("{}", upstream_message(.0))]
    Upstream(#[source] SourceError),
}

fn upstream_message(error: &SourceError) -> String {
    match error {
        SourceError::UmaMoe(_) => "could not reach uma.moe".into(),
        other => other.summary(),
    }
}

type Entries = HashMap<(u64, Option<Period>), (Instant, Outcome)>;

/// A looked-up trainer: their pace and the club it was found in.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrainerPace {
    /// Daily fan gain.
    pub pace: f64,
    /// The trainer's club, if the lookup found one.
    pub circle_id: Option<u64>,
}

/// Days of data this month from which a trainer's pace no longer blends in last month's.
pub const TRAINER_BLEND_DAYS: usize = 10;

/// How long a trainer's pace leans on last month's: `k` in the blend.
pub const TRAINER_PRIOR_K: f64 = 4.0;

/// Last month's pace is left out when this month's is more than this many times higher or lower.
pub const MAX_PRIOR_RATIO: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Outcome {
    /// The trainer's snapshot, whatever its amount of data.
    Fans(Fans),
    Pace(TrainerPace),
    NotFound,
    NotEnoughData,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Fans {
    daily_fans: Vec<i64>,
    period: Option<Period>,
    circle_id: Option<u64>,
}

impl Outcome {
    fn into_result(self) -> Result<TrainerPace, PaceError> {
        match self {
            Self::Fans(fans) => match daily_pace(&fans.daily_fans) {
                Some(pace) if pace > 0.0 => Ok(TrainerPace {
                    pace,
                    circle_id: fans.circle_id,
                }),
                _ => Err(PaceError::NotEnoughData),
            },
            Self::Pace(found) => Ok(found),
            Self::NotFound => Err(PaceError::NotFound),
            Self::NotEnoughData => Err(PaceError::NotEnoughData),
        }
    }

    fn has_pace(&self) -> bool {
        self.clone().into_result().is_ok()
    }
}

/// Looks up trainers' paces, reusing results for a while. Clones share the cache.
#[derive(Debug, Clone)]
pub struct PaceLookup {
    source: Source,
    ttl: Duration,
    entries: Arc<Mutex<Entries>>,
    shared: SharedCache,
    details: Option<ClubDetails>,
}

impl PaceLookup {
    /// Reuses each result for `ttl`.
    pub fn new(source: impl Into<Source>, ttl: Duration) -> Self {
        Self {
            source: source.into(),
            ttl,
            entries: Arc::default(),
            shared: SharedCache::none(),
            details: None,
        }
    }

    /// Checks `shared` before the source, and stores each result in it.
    pub fn with_shared(self, shared: SharedCache) -> Self {
        Self { shared, ..self }
    }

    /// Reads trainers' clubs through `details`, so cached clubs don't need another source call.
    pub fn with_details(self, details: ClubDetails) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }

    /// The trainer's daily fan gain and club this month, or in `period` if given. For a given
    /// period it falls back to the 7-day gain ÷ 7 when that month has too little data.
    ///
    /// Found and not-found results are cached per month.
    pub async fn pace(
        &self,
        viewer_id: u64,
        period: Option<Period>,
    ) -> Result<TrainerPace, PaceError> {
        let span = in_trace(|| tracing::info_span!("pace lookup", source = Empty));
        self.find(viewer_id, period).instrument(span).await
    }

    /// The trainer's pace this month, blended with last month's while this month has fewer than
    /// [`TRAINER_BLEND_DAYS`] days of data; see the README. Without data this month, it's the pace
    /// in `fallback`, if given.
    pub async fn blended(
        &self,
        viewer_id: u64,
        fallback: Option<Period>,
    ) -> Result<TrainerPace, PaceError> {
        let span = in_trace(|| tracing::info_span!("pace lookup", source = Empty));
        self.blend(viewer_id, fallback).instrument(span).await
    }

    async fn blend(
        &self,
        viewer_id: u64,
        fallback: Option<Period>,
    ) -> Result<TrainerPace, PaceError> {
        let fans = match self.outcome(viewer_id, None).await? {
            Outcome::Fans(fans) => fans,
            Outcome::NotFound | Outcome::NotEnoughData if fallback.is_some() => {
                return self.find(viewer_id, fallback).await;
            }
            other => return other.into_result(),
        };
        let days = fans.daily_fans.iter().filter(|v| **v != 0).count();
        let previous = fans.period.map(month_before).or(fallback);
        let (prior, prior_error) = match previous {
            Some(previous) if days < TRAINER_BLEND_DAYS => {
                match self.find(viewer_id, Some(previous)).await {
                    Ok(found) => (Some(found.pace), None),
                    Err(PaceError::Upstream(error)) => (None, Some(error)),
                    Err(_) => (None, None),
                }
            }
            _ => (None, None),
        };
        // A pace that clearly changed since last month isn't held back by it.
        let prior = prior.filter(|prior| match daily_pace(&fans.daily_fans) {
            Some(pace) => (prior / MAX_PRIOR_RATIO..=prior * MAX_PRIOR_RATIO).contains(&pace),
            None => true,
        });
        match blended_pace(&fans.daily_fans, prior, TRAINER_PRIOR_K) {
            Some(pace) if pace > 0.0 => Ok(TrainerPace {
                pace,
                circle_id: fans.circle_id,
            }),
            _ => Err(prior_error.map_or(PaceError::NotEnoughData, PaceError::Upstream)),
        }
    }

    /// The trainer's cumulative fans per day this month, or in `period` if given. `None` if they
    /// aren't found or their pace came from the 7-day gain. Shares the cache with [`Self::pace`].
    pub async fn daily_fans(
        &self,
        viewer_id: u64,
        period: Option<Period>,
    ) -> Result<Option<Vec<i64>>, PaceError> {
        Ok(match self.outcome(viewer_id, period).await? {
            Outcome::Fans(fans) => Some(fans.daily_fans),
            _ => None,
        })
    }

    async fn find(&self, viewer_id: u64, period: Option<Period>) -> Result<TrainerPace, PaceError> {
        self.outcome(viewer_id, period).await?.into_result()
    }

    async fn outcome(&self, viewer_id: u64, period: Option<Period>) -> Result<Outcome, PaceError> {
        if let Some(outcome) = self.cached(viewer_id, period) {
            count_pace("memory");
            return Ok(outcome);
        }
        let key = self.shared.pace_key(viewer_id, period);
        if let Some(outcome) = self.shared.get::<Outcome>(&key).await {
            count_pace("redis");
            self.store(viewer_id, period, outcome.clone());
            return Ok(outcome);
        }

        let outcome = self.lookup(viewer_id, period).await;
        count_pace(if outcome.is_ok() {
            self.source.name()
        } else {
            "error"
        });
        let outcome = outcome?;
        self.store(viewer_id, period, outcome.clone());
        self.shared.set(&key, &outcome, self.ttl).await;
        Ok(outcome)
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
        match self.source.club(&request, Priority::Interactive).await {
            Ok(response) => Ok(pace_from_response(&response, viewer_id)),
            Err(error) if error.is_not_found() => Ok(Outcome::NotFound),
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
            Some(_) if !outcome.has_pace() => self.seven_day_gains(viewer_id, outcome).await,
            _ => Ok(outcome),
        }
    }

    async fn ranked_club(&self, viewer_id: u64, period: Option<Period>) -> Option<u64> {
        let mut request = RankingsMonthlyRequest::for_viewer(viewer_id);
        if let Some((year, month)) = period {
            request = request.for_month(year, month);
        }
        match self.source.rankings_monthly(&request).await {
            Ok(rankings) => rankings.viewer(viewer_id).and_then(|row| row.circle_id),
            Err(error) if error.is_not_found() => None,
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
                        Some(circle_id),
                    )
                })
                .map_err(|error| (error.origin(), error.to_string())),
            _ => {
                let mut request = CircleRequest::by_circle_id(circle_id);
                if let Some((year, month)) = period {
                    request = request.for_month(year, month);
                }
                self.source
                    .club(&request, Priority::Interactive)
                    .await
                    .map(|response| pace_from_response(&response, viewer_id))
                    .map(|outcome| with_club(outcome, circle_id))
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
            .source
            .rankings_gains(&RankingsGainsRequest::for_viewer(viewer_id))
            .await
        {
            Ok(gains) => gains,
            Err(error) if error.is_not_found() => return Ok(outcome),
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
        let Some(row) = gains.viewer(viewer_id) else {
            return Ok(outcome);
        };
        Ok(match row.gain_7d {
            Some(gain) if gain > 0 => Outcome::Pace(TrainerPace {
                pace: gain as f64 / 7.0,
                circle_id: row.circle_id,
            }),
            _ => outcome,
        })
    }

    fn cached(&self, viewer_id: u64, period: Option<Period>) -> Option<Outcome> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries
            .get(&(viewer_id, period))
            .filter(|(at, _)| at.elapsed() < self.ttl)
            .map(|(_, outcome)| outcome.clone())
    }

    fn store(&self, viewer_id: u64, period: Option<Period>, outcome: Outcome) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|_, (at, _)| at.elapsed() < self.ttl);
        entries.insert((viewer_id, period), (Instant::now(), outcome));
    }
}

fn with_club(outcome: Outcome, circle_id: u64) -> Outcome {
    match outcome {
        Outcome::Fans(fans) => Outcome::Fans(Fans {
            circle_id: Some(circle_id),
            ..fans
        }),
        Outcome::Pace(found) => Outcome::Pace(TrainerPace {
            circle_id: Some(circle_id),
            ..found
        }),
        other => other,
    }
}

fn count_pace(source: &'static str) {
    metrics::counter!("pace_lookups_total", "source" => source).increment(1);
    Span::current().record("source", source);
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
        Some(response.circle.circle_id).filter(|id| *id != 0),
    )
}

fn pace_of<'a>(
    members: impl Iterator<Item = (u64, Option<i32>, Option<i32>, &'a [i64])>,
    viewer_id: u64,
    circle_id: Option<u64>,
) -> Outcome {
    let snapshot = members
        .filter(|(id, ..)| *id == viewer_id)
        .max_by_key(|(_, year, month, _)| (*year, *month));
    match snapshot {
        None => Outcome::NotFound,
        Some((_, year, month, daily_fans)) => Outcome::Fans(Fans {
            daily_fans: daily_fans.to_vec(),
            period: year.zip(month.and_then(|m| u32::try_from(m).ok())),
            circle_id,
        }),
    }
}

#[cfg(test)]
mod tests;
