//! HTTP routes.

use std::path::Path;
use std::time::{Duration, SystemTime};

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::catch_panic::CatchPanicLayer;

use crate::api_error::ApiError;
use crate::cache::{Cache, ScanFailure, ScanPhase, Snapshot};
use crate::details::ClubDetails;
use crate::frontend;
use crate::human::{challenge, HumanCheck};
use crate::limits::{limit_requests, RateLimits};
use crate::pace::PaceLookup;
use crate::recommendations::{recommendations, MatchSettings};
use crate::telemetry::track_requests;

/// Longest a request may run before it gets a 504. Matches the longest `Retry-After` pause a
/// visitor's request waits out.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Shared state for request handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Scan results.
    pub cache: Cache,
    /// Trainer pace lookups.
    pub paces: PaceLookup,
    /// Per-visitor rate limits.
    pub limits: RateLimits,
    /// Cached club details and the club pool.
    pub details: ClubDetails,
    /// Shortlist and fetch settings for recommendations.
    pub matching: MatchSettings,
    /// The human check.
    pub human: HumanCheck,
}

/// The server's routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/recommendations",
            get(recommendations).route_layer(from_fn_with_state(state.clone(), limit_requests)),
        )
        .route(
            "/api/v1/challenge",
            post(challenge).route_layer(from_fn_with_state(state.clone(), limit_requests)),
        )
        .route("/api", any(api_not_found))
        .route("/api/{*rest}", any(api_not_found))
        .with_state(state)
}

/// The routes plus security headers, a request timeout and a JSON 500 for panics, serving the built
/// frontend from `frontend_dir` if given.
pub fn app(state: AppState, frontend_dir: Option<&Path>) -> Router {
    let mut app = router(state);
    if let Some(dir) = frontend_dir {
        app = app.fallback_service(frontend::service(dir));
    }
    app.layer(CatchPanicLayer::custom(panic_response))
        .layer(from_fn(time_limit))
        .layer(from_fn(track_requests))
        .layer(from_fn(frontend::security_headers))
}

/// Answers a request whose handler panicked. The panic itself is logged by the panic hook.
fn panic_response(_: Box<dyn std::any::Any + Send>) -> Response {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
}

/// Answers 504 with a JSON error if the request runs longer than [`REQUEST_TIMEOUT`].
async fn time_limit(request: Request, next: Next) -> Response {
    tokio::time::timeout(REQUEST_TIMEOUT, next.run(request))
        .await
        .unwrap_or_else(|_| {
            ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "the request took too long; try again",
            )
            .into_response()
        })
}

async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Serves `GET /status`: the full scan, pool, cache and refresh state. Serve it only on a private
/// address, as [`metrics_router`](crate::metrics_router) does.
pub fn status_router(state: AppState) -> Router {
    Router::new()
        .route("/status", get(full_status))
        .with_state(state)
}

async fn full_status(State(state): State<AppState>) -> Json<Value> {
    let status = state.cache.status();
    let refresh = state.details.refresh_status().await;
    let (pool_clubs, exact_medians, calibration) = {
        let pool = state
            .details
            .pool()
            .read()
            .unwrap_or_else(|e| e.into_inner());
        (pool.len(), pool.exact_medians(), pool.calibration())
    };
    let scan = status.progress.map(|p| {
        json!({
            "phase": match p.phase {
                ScanPhase::Listing => "listing",
                ScanPhase::Loading => "loading",
                ScanPhase::Waiting => "waiting",
            },
            "started_at": p.started_at.map(timestamp),
            "listed": p.listed,
            "to_load": p.to_load,
            "loaded": p.loaded,
            "failed": p.failed,
        })
    });
    Json(json!({
        "scanning": status.scanning,
        "scan": scan,
        "last_scan": status.snapshot.as_deref().map(last_scan),
        "pool": {
            "clubs": pool_clubs,
            "exact_medians": exact_medians,
            "calibration": {
                "factor": calibration.factor(),
                "samples": calibration.samples(),
            },
        },
        "details": {
            "cached": state.details.cached_clubs(),
            "queued": state.details.queued(),
        },
        "refresh": {
            "scheduled": refresh.scheduled,
            "loaded": refresh.loaded,
            "due": refresh.due,
            "oldest_due_at": refresh.oldest_due.map(|due| timestamp(due.into())),
            "full": {
                "scheduled": refresh.full.scheduled,
                "loaded": refresh.full.loaded,
                "due": refresh.full.due,
            },
            "lower": {
                "scheduled": refresh.lower.scheduled,
                "loaded": refresh.lower.loaded,
                "due": refresh.lower.due,
            },
        },
        "last_error": status.last_error.as_ref().map(last_error),
    }))
}

fn last_scan(snapshot: &Snapshot) -> Value {
    json!({
        "finished_at": timestamp(snapshot.scanned_at),
        "clubs": snapshot.prefetched,
        "candidates": snapshot.candidates,
        "failed": snapshot.failed,
        "month": snapshot.month,
    })
}

fn last_error(failure: &ScanFailure) -> Value {
    json!({
        "at": timestamp(failure.at),
        "message": failure.message,
    })
}

pub(crate) fn timestamp(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}

#[cfg(test)]
mod tests;
