use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::Clock;
use governor::{DefaultKeyedRateLimiter, Quota};

use crate::api_error::ApiError;
use crate::app::AppState;

const PRUNE_ABOVE: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Visitor(IpAddr);

#[derive(Clone)]
pub struct RateLimits {
    requests: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    lookups: Arc<DefaultKeyedRateLimiter<IpAddr>>,
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
    pub fn new(
        requests_per_minute: u32,
        lookups_per_minute: u32,
        trust_forwarded_for: bool,
    ) -> Self {
        let quota = |n: u32| Quota::per_minute(NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN));
        Self {
            requests: Arc::new(DefaultKeyedRateLimiter::keyed(quota(requests_per_minute))),
            lookups: Arc::new(DefaultKeyedRateLimiter::keyed(quota(lookups_per_minute))),
            trust_forwarded_for,
        }
    }

    pub(crate) fn check_lookup(&self, visitor: Visitor) -> Result<(), ApiError> {
        check(&self.lookups, visitor)
    }
}

fn check(limiter: &DefaultKeyedRateLimiter<IpAddr>, visitor: Visitor) -> Result<(), ApiError> {
    if limiter.len() > PRUNE_ABOVE {
        limiter.retain_recent();
    }
    limiter.check_key(&visitor.0).map_err(|not_until| {
        ApiError::rate_limited(not_until.wait_time_from(limiter.clock().now()))
    })
}

pub(crate) async fn limit_requests(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let visitor = visitor(&request, state.limits.trust_forwarded_for);
    if let Err(error) = check(&state.limits.requests, visitor) {
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
