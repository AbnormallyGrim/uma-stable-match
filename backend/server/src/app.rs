//! HTTP routes.

use std::time::SystemTime;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::cache::Cache;
use crate::pace::PaceLookup;
use crate::recommendations::recommendations;

#[derive(Debug, Clone)]
pub struct AppState {
    pub cache: Cache,
    pub paces: PaceLookup,
}

/// The server's routes, reading scan results from `cache`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/recommendations", get(recommendations))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let status = state.cache.status();
    Json(json!({
        "scanning": status.scanning,
        "last_scan": status.snapshot.map(|s| json!({
            "finished_at": timestamp(s.scanned_at),
            "clubs": s.clubs.len(),
            "candidates": s.candidates,
            "failed": s.failed,
        })),
        "last_error": status.last_error.map(|e| json!({
            "at": timestamp(e.at),
            "message": e.message,
        })),
    }))
}

pub(crate) fn timestamp(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}
