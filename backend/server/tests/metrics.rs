//! Metrics tests. They share one global recorder, so counts add up across tests. All data is made
//! up.

use std::sync::OnceLock;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::PrometheusHandle;
use tower::ServiceExt;
use uma_stable_match_server::{
    app, install_metrics, metrics_router, AppState, Cache, ClubDetails, DetailConfig,
    MatchSettings, PaceLookup, RateLimits, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};

static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

fn handle() -> &'static PrometheusHandle {
    HANDLE.get_or_init(|| install_metrics().unwrap())
}

fn client(base_url: String) -> Client {
    Client::new(ClientConfig {
        base_url,
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 1,
            base_delay: Duration::from_millis(1),
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn state() -> AppState {
    let client = client("http://127.0.0.1:9".into());
    AppState {
        cache: Cache::new(),
        details: ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
        limits: RateLimits::new(1, 10_000, false),
    }
}

fn has(rendered: &str, name: &str, labels: &[&str]) -> bool {
    rendered.lines().any(|line| {
        line.starts_with(&format!("{name}{{")) && labels.iter().all(|l| line.contains(l))
    })
}

async fn get(router: axum::Router, uri: &str) -> StatusCode {
    router
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn requests_are_counted_by_route_and_status() {
    let handle = handle();
    let app = app(state(), None);
    assert_eq!(get(app.clone(), "/api/v1/health").await, StatusCode::OK);
    assert_eq!(
        get(app.clone(), "/api/v1/made-up").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(app.clone(), "/api/v1/recommendations?pace=1000000").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        get(app, "/api/v1/recommendations?pace=1000000").await,
        StatusCode::TOO_MANY_REQUESTS
    );

    let rendered = handle.render();
    assert!(has(
        &rendered,
        "http_requests_total",
        &[
            r#"route="/api/v1/health""#,
            r#"method="GET""#,
            r#"status="200""#
        ]
    ));
    assert!(has(
        &rendered,
        "http_requests_total",
        &[r#"route="/api/{*rest}""#, r#"status="404""#]
    ));
    assert!(has(
        &rendered,
        "http_requests_total",
        &[r#"route="/api/v1/recommendations""#, r#"status="503""#]
    ));
    assert!(has(
        &rendered,
        "http_requests_total",
        &[r#"route="/api/v1/recommendations""#, r#"status="429""#]
    ));
    assert!(has(
        &rendered,
        "rate_limited_total",
        &[r#"limit="requests""#]
    ));
    assert!(rendered.contains("http_request_duration_seconds_bucket{"));
}

#[tokio::test]
async fn the_metrics_route_reports_the_pool_and_scan() {
    let handle = handle();
    let response = metrics_router(handle.clone(), state())
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rendered = String::from_utf8(body.to_vec()).unwrap();
    for name in ["pool_clubs", "pool_calibration_factor", "scan_running"] {
        assert!(
            rendered.lines().any(|l| l.starts_with(&format!("{name} "))),
            "missing {name}"
        );
    }
    assert!(has(&rendered, "build_info", &[r#"version=""#]));
}
