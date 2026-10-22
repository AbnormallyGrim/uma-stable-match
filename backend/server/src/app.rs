//! HTTP routes.

use std::path::Path;
use std::time::SystemTime;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::IntoResponse;
use axum::routing::{any, get};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::cache::{Cache, ScanFailure, ScanPhase, Snapshot};
use crate::details::ClubDetails;
use crate::frontend;
use crate::limits::{limit_requests, RateLimits};
use crate::pace::PaceLookup;
use crate::recommendations::{recommendations, MatchSettings};
use crate::telemetry::track_requests;

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
}

/// The server's routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/recommendations",
            get(recommendations).route_layer(from_fn_with_state(state.clone(), limit_requests)),
        )
        .route("/api", any(api_not_found))
        .route("/api/{*rest}", any(api_not_found))
        .with_state(state)
}

/// The routes plus security headers, serving the built frontend from `frontend_dir` if given.
pub fn app(state: AppState, frontend_dir: Option<&Path>) -> Router {
    let mut app = router(state);
    if let Some(dir) = frontend_dir {
        app = app.fallback_service(frontend::service(dir));
    }
    app.layer(from_fn(track_requests))
        .layer(from_fn(frontend::security_headers))
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
