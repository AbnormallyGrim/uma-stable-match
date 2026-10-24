//! Optional Prometheus metrics, served apart from the app.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::{BuildError, PrometheusBuilder, PrometheusHandle};

use crate::app::{status_router, AppState};
use crate::refresher::utc_now;

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
    Ok(handle)
}

/// Serves `GET /metrics`, refreshing the scan and pool values from `state` on each request, and
/// `GET /status` from [`status_router`](crate::status_router).
pub fn metrics_router(handle: PrometheusHandle, state: AppState) -> Router {
    let statuses = status_router(state.clone());
    Router::new()
        .route(
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
        .merge(statuses)
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
    if let Some(snapshot) = state.cache.snapshot() {
        metrics::gauge!("scan_last_success_timestamp_seconds")
            .set(unix_seconds(snapshot.scanned_at));
    }

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

    state.details.settle().await;
    metrics::gauge!("detail_cache_clubs").set(state.details.cached_clubs() as f64);
    metrics::gauge!("detail_queue_clubs").set(state.details.queued() as f64);

    let refresh = state.details.refresh_status().await;
    metrics::gauge!("refresh_scheduled_clubs").set(refresh.scheduled as f64);
    metrics::gauge!("refresh_loaded_clubs").set(refresh.loaded as f64);
    metrics::gauge!("refresh_due_clubs").set(refresh.due as f64);
    let overdue = refresh.oldest_due.map_or(0.0, |due| {
        utc_now()
            .signed_duration_since(due)
            .to_std()
            .unwrap_or_default()
            .as_secs_f64()
    });
    metrics::gauge!("refresh_overdue_seconds").set(overdue);
    if let Some(ttl) = refresh.stored_ttl {
        metrics::gauge!("redis_club_ttl_shortest_seconds").set(ttl.shortest.as_secs_f64());
        metrics::gauge!("redis_club_ttl_average_seconds").set(ttl.average.as_secs_f64());
    }

    if let Some(stats) = state.details.shared().stats().await {
        metrics::gauge!("redis_keys").set(stats.keys as f64);
        metrics::gauge!("redis_keys_expiring").set(stats.expiring as f64);
        metrics::gauge!("redis_average_ttl_seconds").set(stats.average_ttl.as_secs_f64());
        metrics::gauge!("redis_memory_used_bytes").set(stats.used_memory as f64);
        metrics::gauge!("redis_memory_max_bytes").set(stats.max_memory as f64);
        metrics::counter!("redis_evicted_keys_total").absolute(stats.evicted);
        metrics::counter!("redis_expired_keys_total").absolute(stats.expired);
    }
}

fn unix_seconds(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
