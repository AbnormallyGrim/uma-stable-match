//! Where club and trainer data comes from.

use std::fmt;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use uma_stable_match_umamoe_client::{
    CircleListResponse, CircleRequest, CircleResponse, Client, Error, GainsRankingsResponse,
    ListCirclesRequest, MonthlyRankingsResponse, Priority, RankingsGainsRequest,
    RankingsMonthlyRequest, StatusCode,
};

/// Longest `Retry-After` pause a visitor's request waits out; longer ones fail straight away.
const VISITOR_MAX_PAUSE: Duration = Duration::from_secs(30);

/// The future a [`ClubSource`] call returns.
pub type SourceFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SourceError>> + Send + 'a>>;

/// Club and trainer data the server reads. See the crate README.
pub trait ClubSource: Send + Sync + 'static {
    /// Short label for metrics and traces, such as `"umamoe"`.
    fn name(&self) -> &'static str;

    /// A club and its members' fan data, found by club or trainer ID. `priority` is the lane a
    /// rate-limited source waits in.
    fn club<'a>(
        &'a self,
        request: &'a CircleRequest,
        priority: Priority,
    ) -> SourceFuture<'a, CircleResponse>;

    /// One page of clubs.
    fn list_clubs<'a>(
        &'a self,
        request: &'a ListCirclesRequest,
    ) -> SourceFuture<'a, CircleListResponse>;

    /// One page of a month's trainer fan rankings.
    fn rankings_monthly<'a>(
        &'a self,
        request: &'a RankingsMonthlyRequest,
    ) -> SourceFuture<'a, MonthlyRankingsResponse>;

    /// One page of trainers' rolling fan gains.
    fn rankings_gains<'a>(
        &'a self,
        request: &'a RankingsGainsRequest,
    ) -> SourceFuture<'a, GainsRankingsResponse>;
}

/// Why a [`ClubSource`] call failed.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// The uma.moe request failed.
    #[error(transparent)]
    UmaMoe(#[from] Error),
    /// The source has no such club or trainer.
    #[error("not found")]
    NotFound,
    /// Another source failed. `message` is shown to visitors, so it must not hold internal details.
    #[error("{message}")]
    Unavailable {
        /// Label for logs and metrics.
        origin: &'static str,
        /// Short, public description.
        message: String,
    },
}

impl SourceError {
    /// Where the error came from, for logs and metrics. uma.moe errors use [`Error::origin`].
    pub fn origin(&self) -> &'static str {
        match self {
            Self::UmaMoe(error) => error.origin(),
            Self::NotFound => "source",
            Self::Unavailable { origin, .. } => origin,
        }
    }

    /// HTTP status, if the source returned one. [`SourceError::NotFound`] counts as 404.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::UmaMoe(error) => error.status(),
            Self::NotFound => Some(StatusCode::NOT_FOUND),
            Self::Unavailable { .. } => None,
        }
    }

    /// Whether the club or trainer doesn't exist.
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(StatusCode::NOT_FOUND)
    }

    /// A short message that's safe to show visitors: no URLs or upstream error text.
    pub(crate) fn summary(&self) -> String {
        match self {
            Self::UmaMoe(error) => match error {
                Error::Status { status, .. } => format!("uma.moe returned {status}"),
                Error::RateLimited { .. } => "uma.moe asked to slow down".into(),
                Error::Transport(_) => "could not reach uma.moe".into(),
                Error::Decode(_) => "unexpected response from uma.moe".into(),
                Error::Config(_) | Error::InvalidRequest(_) => "internal error".into(),
            },
            Self::NotFound => "not found".into(),
            Self::Unavailable { message, .. } => message.clone(),
        }
    }
}

/// A shared [`ClubSource`]. Clones share it.
#[derive(Clone)]
pub struct Source(Arc<dyn ClubSource>);

impl Source {
    /// Wraps `source` for sharing.
    pub fn new(source: impl ClubSource) -> Self {
        Self(Arc::new(source))
    }
}

impl Deref for Source {
    type Target = dyn ClubSource;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Source").finish_non_exhaustive()
    }
}

impl From<&Source> for Source {
    fn from(source: &Source) -> Self {
        source.clone()
    }
}

impl From<UmaMoeSource> for Source {
    fn from(source: UmaMoeSource) -> Self {
        Self::new(source)
    }
}

impl From<&Client> for Source {
    fn from(client: &Client) -> Self {
        Self::new(UmaMoeSource::new(client))
    }
}

impl From<Client> for Source {
    fn from(client: Client) -> Self {
        Self::from(&client)
    }
}

/// [`ClubSource`] backed by the uma.moe API.
#[derive(Debug, Clone)]
pub struct UmaMoeSource {
    listing: Client,
    visitor: Client,
    background: Client,
}

impl UmaMoeSource {
    /// Uses clones of `client`, which share its rate limit. Visitor requests give up on long
    /// `Retry-After` pauses.
    pub fn new(client: &Client) -> Self {
        Self {
            listing: client.clone(),
            visitor: client
                .with_priority(Priority::Interactive)
                .with_max_pause(VISITOR_MAX_PAUSE),
            background: client.with_priority(Priority::Background),
        }
    }
}

impl ClubSource for UmaMoeSource {
    fn name(&self) -> &'static str {
        "umamoe"
    }

    fn club<'a>(
        &'a self,
        request: &'a CircleRequest,
        priority: Priority,
    ) -> SourceFuture<'a, CircleResponse> {
        let client = match priority {
            Priority::Interactive => &self.visitor,
            Priority::Background => &self.background,
        };
        Box::pin(async move { Ok(client.circle(request).await?) })
    }

    fn list_clubs<'a>(
        &'a self,
        request: &'a ListCirclesRequest,
    ) -> SourceFuture<'a, CircleListResponse> {
        Box::pin(async move { Ok(self.listing.list_circles(request).await?) })
    }

    fn rankings_monthly<'a>(
        &'a self,
        request: &'a RankingsMonthlyRequest,
    ) -> SourceFuture<'a, MonthlyRankingsResponse> {
        Box::pin(async move { Ok(self.visitor.rankings_monthly(request).await?) })
    }

    fn rankings_gains<'a>(
        &'a self,
        request: &'a RankingsGainsRequest,
    ) -> SourceFuture<'a, GainsRankingsResponse> {
        Box::pin(async move { Ok(self.visitor.rankings_gains(request).await?) })
    }
}

#[cfg(test)]
mod tests;
