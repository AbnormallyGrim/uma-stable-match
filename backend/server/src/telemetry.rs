//! Optional Prometheus metrics, served apart from the app.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::{BuildError, PrometheusBuilder, PrometheusHandle};

use crate::app::AppState;

const BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0, 900.0,
    1800.0, 3600.0,
];
const UPKEEP_EVERY: Duration = Duration::from_secs(5);

/// Installs the global metrics recorder and returns the handle that renders it. Call it once,
/// inside a Tokio runtime.
pub fn install_metrics() -> Result<PrometheusHandle, BuildError> {
    let handle = PrometheusBuilder::new()
        .set_buckets(BUCKETS)?
        .install_recorder()?;
    let upkeep = handle.clone();
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(UPKEEP_EVERY);
        loop {
            ticks.tick().await;
            upkeep.run_upkeep();
        }
    });
    metrics::gauge!("build_info", "version" => env!("CARGO_PKG_VERSION")).set(1.0);
    Ok(handle)
}

/// Serves `GET /metrics`, refreshing the scan and pool values from `state` on each request.
pub fn metrics_router(handle: PrometheusHandle, state: AppState) -> Router {
    Router::new().route(
        "/metrics",
        get(move || {
            let handle = handle.clone();
            let state = state.clone();
            async move {
                record_state(&state).await;
                handle.render()
            }
        }),
    )
}

pub(crate) async fn track_requests(request: Request, next: Next) -> Response {
    let route = route_label(&request);
    let method = request.method().as_str().to_owned();
    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    metrics::histogram!("http_request_duration_seconds", "route" => route.clone(), "method" => method.clone())
        .record(started.elapsed().as_secs_f64());
    metrics::counter!("http_requests_total", "route" => route, "method" => method, "status" => status)
        .increment(1);
    response
}

fn route_label(request: &Request) -> String {
    if let Some(path) = request.extensions().get::<MatchedPath>() {
        return path.as_str().to_owned();
    }
    if request.uri().path().starts_with("/api") {
        "/api (unknown)".to_owned()
    } else {
        "frontend".to_owned()
    }
}

async fn record_state(state: &AppState) {
    let status = state.cache.status();
    metrics::gauge!("scan_running").set(f64::from(u8::from(status.scanning)));
    if let Some(snapshot) = &status.snapshot {
        metrics::gauge!("scan_last_success_timestamp_seconds")
            .set(unix_seconds(snapshot.scanned_at));
        metrics::gauge!("last_scan_candidates").set(snapshot.candidates as f64);
        metrics::gauge!("last_scan_clubs_loaded").set(snapshot.prefetched as f64);
        metrics::gauge!("last_scan_clubs_failed").set(snapshot.failed as f64);
    }
    metrics::gauge!("scan_last_error").set(f64::from(u8::from(status.last_error.is_some())));

    let (clubs, exact, factor, samples) = {
        let pool = state
            .details
            .pool()
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let calibration = pool.calibration();
        (
            pool.len(),
            pool.exact_medians(),
            calibration.factor(),
            calibration.samples(),
        )
    };
    metrics::gauge!("pool_clubs").set(clubs as f64);
    metrics::gauge!("pool_exact_medians").set(exact as f64);
    metrics::gauge!("pool_calibration_factor").set(factor);
    metrics::gauge!("pool_calibration_samples").set(samples as f64);
}

fn unix_seconds(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
