use std::time::SystemTime;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::cache::Cache;

pub fn router(cache: Cache) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .with_state(cache)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn status(State(cache): State<Cache>) -> Json<Value> {
    let status = cache.status();
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

fn timestamp(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}
