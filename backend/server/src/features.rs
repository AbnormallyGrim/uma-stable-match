//! Feature flags the frontend reads at startup.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

/// Features that are switched on. All are off by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Features {
    /// Whether the Japanese translation and the EN/JP switch are offered.
    pub japanese: bool,
}

pub(crate) fn router(features: Features) -> Router {
    Router::new()
        .route("/api/v1/features", get(show))
        .with_state(features)
}

async fn show(State(features): State<Features>) -> Json<Value> {
    Json(json!({ "japanese": features.japanese }))
}
