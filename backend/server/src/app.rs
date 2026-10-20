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

use crate::cache::Cache;
use crate::details::ClubDetails;
use crate::frontend;
use crate::limits::{limit_requests, RateLimits};
use crate::pace::PaceLookup;
use crate::recommendations::recommendations;

/// Shared state for request handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Scan results.
    pub cache: Cache,
    /// Trainer pace lookups.
    pub paces: PaceLookup,
    /// Per-visitor rate limits.
    pub limits: RateLimits,
    pub details: ClubDetails,
}

/// The server's routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/status", get(status))
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
    app.layer(from_fn(frontend::security_headers))
}

async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let status = state.cache.status();
    let (pool_clubs, exact_medians, calibration) = {
        let pool = state
            .details
            .pool()
            .read()
            .unwrap_or_else(|e| e.into_inner());
        (pool.len(), pool.exact_medians(), pool.calibration())
    };
    Json(json!({
        "scanning": status.scanning,
        "last_scan": status.snapshot.map(|s| json!({
            "finished_at": timestamp(s.scanned_at),
            "clubs": s.prefetched,
            "candidates": s.candidates,
            "failed": s.failed,
            "month": s.month,
        })),
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
        "last_error": status.last_error.map(|e| json!({
            "at": timestamp(e.at),
            "message": e.message,
        })),
    }))
}

pub(crate) fn timestamp(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}
