//! Per-visitor rate limits.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::ACCEPT;
use axum::http::HeaderMap;
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Router;
use governor::clock::Clock;
use governor::{DefaultKeyedRateLimiter, Quota};
use uma_stable_match_discord::{
    APPLICATIONS_PAGE, APPLY_PATH, CALLBACK_PATH, INTERACTIONS_PATH, OPT_IN_PATH,
};

use crate::api_error::ApiError;
use crate::app::AppState;
use crate::config::{DEFAULT_DISCORD_REQUESTS_PER_MINUTE, DEFAULT_DISCORD_SIGN_INS_PER_HOUR};

// Forget idle visitors once this many are tracked.
const PRUNE_ABOVE: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Visitor(IpAddr);

/// Per-visitor request limits. Clones share the same counters.
#[derive(Clone)]
pub struct RateLimits {
    requests: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    lookups: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    discord_requests: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    discord_sign_ins: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    trust_forwarded_for: bool,
}

impl fmt::Debug for RateLimits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RateLimits")
            .field("trust_forwarded_for", &self.trust_forwarded_for)
            .finish_non_exhaustive()
    }
}

impl RateLimits {
    /// Limits per minute for all recommendation requests and for uncached trainer lookups, with the
    /// default Discord limits.
    ///
    /// With `trust_forwarded_for`, visitors are identified by the right-most `X-Forwarded-For` address.
    pub fn new(
        requests_per_minute: u32,
        lookups_per_minute: u32,
        trust_forwarded_for: bool,
    ) -> Self {
        Self {
            requests: keyed(Quota::per_minute(at_least_one(requests_per_minute))),
            lookups: keyed(Quota::per_minute(at_least_one(lookups_per_minute))),
            trust_forwarded_for,
            discord_requests: keyed(Quota::per_minute(at_least_one(
                DEFAULT_DISCORD_REQUESTS_PER_MINUTE,
            ))),
            discord_sign_ins: keyed(Quota::per_hour(at_least_one(
                DEFAULT_DISCORD_SIGN_INS_PER_HOUR,
            ))),
        }
    }

    /// The same limits, with `requests_per_minute` for the Discord routes and `sign_ins_per_hour`
    /// for the ones that go to or come back from Discord. See [`limit_discord`].
    pub fn with_discord(self, requests_per_minute: u32, sign_ins_per_hour: u32) -> Self {
        Self {
            discord_requests: keyed(Quota::per_minute(at_least_one(requests_per_minute))),
            discord_sign_ins: keyed(Quota::per_hour(at_least_one(sign_ins_per_hour))),
            ..self
        }
    }

    pub(crate) fn check_lookup(&self, visitor: Visitor) -> Result<(), ApiError> {
        check(&self.lookups, visitor, "trainer_lookups")
    }
}

fn keyed(quota: Quota) -> Arc<DefaultKeyedRateLimiter<IpAddr>> {
    Arc::new(DefaultKeyedRateLimiter::keyed(quota))
}

fn at_least_one(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN)
}

/// `routes`, the Discord routes, behind per-visitor limits: the sign-in callback and starting an
/// application or an opt-in share the hourly limit, and everything else the per-minute one. The
/// interactions endpoint isn't limited, as Discord sends it and signs every request.
pub fn limit_discord(routes: Router, limits: RateLimits) -> Router {
    routes.layer(from_fn_with_state(limits, discord_limit))
}

async fn discord_limit(State(limits): State<RateLimits>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let (limiter, label) = if path == INTERACTIONS_PATH {
        return next.run(request).await;
    } else if [CALLBACK_PATH, APPLY_PATH, OPT_IN_PATH].contains(&path) {
        (&limits.discord_sign_ins, "discord_sign_ins")
    } else {
        (&limits.discord_requests, "discord_requests")
    };
    let visitor = visitor(&request, limits.trust_forwarded_for);
    match check(limiter, visitor, label) {
        Ok(()) => next.run(request).await,
        // A page load lands on the applications page, which explains it.
        Err(_) if wants_html(request.headers()) => {
            Redirect::to(&format!("{APPLICATIONS_PAGE}?error=rate_limited")).into_response()
        }
        Err(error) => error.into_response(),
    }
}

fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"))
}

fn check(
    limiter: &DefaultKeyedRateLimiter<IpAddr>,
    visitor: Visitor,
    limit: &'static str,
) -> Result<(), ApiError> {
    if limiter.len() > PRUNE_ABOVE {
        limiter.retain_recent();
    }
    limiter.check_key(&visitor.0).map_err(|not_until| {
        metrics::counter!("rate_limited_total", "limit" => limit).increment(1);
        ApiError::rate_limited(not_until.wait_time_from(limiter.clock().now()))
    })
}

pub(crate) async fn limit_requests(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let visitor = visitor(&request, state.limits.trust_forwarded_for);
    if let Err(error) = check(&state.limits.requests, visitor, "requests") {
        return error.into_response();
    }
    request.extensions_mut().insert(visitor);
    next.run(request).await
}

fn visitor(request: &Request, trust_forwarded_for: bool) -> Visitor {
    let forwarded = if trust_forwarded_for {
        forwarded_for(request.headers())
    } else {
        None
    };
    let ip = forwarded
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0.ip())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    Visitor(group(ip))
}

fn forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get_all("x-forwarded-for")
        .iter()
        .next_back()?
        .to_str()
        .ok()?
        .rsplit(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

// IPv6 visitors are grouped by /64, the usual size of one home network.
fn group(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
            }
        },
    }
}

#[cfg(test)]
mod tests;
