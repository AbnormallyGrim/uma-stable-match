//! Metrics tests. They share one global recorder, so counts add up across tests. All data is made
//! up.

use std::sync::OnceLock;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::PrometheusHandle;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tower::ServiceExt;
use uma_stable_match_server::{
    app, install_metrics, metrics_router, scan, AppState, Cache, ClubDetails, DetailConfig,
    KeySecret, MatchSettings, PaceLookup, RateLimits, RedisUrl, ScanConfig, SharedCache,
    SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{
    ApiKey, Client, ClientConfig, ListCirclesRequest, Priority, RetryConfig,
};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
        human: Default::default(),
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
async fn umamoe_calls_are_counted_by_status_including_retries() {
    let handle = handle();
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": [] })))
        .mount(&server)
        .await;

    client(server.uri())
        .list_circles(&ListCirclesRequest::default())
        .await
        .unwrap();

    let rendered = handle.render();
    for status in ["503", "200"] {
        assert!(has(
            &rendered,
            "umamoe_requests_total",
            &[
                r#"endpoint="/api/v4/circles/list""#,
                r#"lane="interactive""#,
                &format!(r#"status="{status}""#),
            ]
        ));
    }
    assert!(rendered.contains("umamoe_queue_wait_seconds_bucket{"));
}

#[tokio::test]
async fn the_metrics_route_reports_the_pool_and_caches() {
    let handle = handle();
    let response = metrics_router(handle.clone(), state())
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rendered = String::from_utf8(body.to_vec()).unwrap();
    for name in [
        "pool_clubs",
        "pool_calibration_factor",
        "detail_cache_clubs",
        "detail_queue_clubs",
        "refresh_scheduled_clubs",
        "refresh_loaded_clubs",
        "refresh_due_clubs",
        "refresh_overdue_seconds",
    ] {
        assert!(
            rendered.lines().any(|l| l.starts_with(&format!("{name} "))),
            "missing {name}"
        );
    }
}

#[tokio::test]
async fn the_metrics_address_also_serves_the_full_status() {
    let response = metrics_router(handle().clone(), state())
        .oneshot(Request::get("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
    for key in [
        "scanning",
        "scan",
        "last_scan",
        "last_error",
        "pool",
        "details",
        "refresh",
    ] {
        assert!(status.get(key).is_some(), "missing {key}");
    }
}

#[tokio::test]
async fn club_details_are_counted_by_source() {
    let handle = handle();
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000201"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": { "circle_id": 900000201, "name": "Metrics Club", "member_count": 20 },
            "members": [],
        })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000202"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let (details, _loader) = ClubDetails::new(
        &client(server.uri()),
        SharedPool::default(),
        DetailConfig::default(),
    );

    details.get(900000201, Priority::Interactive).await.unwrap();
    details.get(900000201, Priority::Interactive).await.unwrap();
    details
        .get(900000202, Priority::Interactive)
        .await
        .unwrap_err();

    let rendered = handle.render();
    for source in ["umamoe", "memory", "error"] {
        assert!(
            has(
                &rendered,
                "club_detail_lookups_total",
                &[&format!(r#"source="{source}""#)]
            ),
            "missing {source}"
        );
    }
}

#[tokio::test]
async fn trainer_paces_are_counted_by_source() {
    let handle = handle();
    let server = MockServer::start().await;
    let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + 3_000_000 * day).collect();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", "100000000301"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": { "circle_id": 900000301 },
            "members": [{ "viewer_id": 100000000301u64, "year": 2026, "month": 3, "daily_fans": fans }],
        })))
        .mount(&server)
        .await;
    let paces = PaceLookup::new(client(server.uri()), DEFAULT_PACE_TTL);

    paces.pace(100000000301, None).await.unwrap();
    paces.pace(100000000301, None).await.unwrap();

    let rendered = handle.render();
    for source in ["umamoe", "memory"] {
        assert!(
            has(
                &rendered,
                "pace_lookups_total",
                &[&format!(r#"source="{source}""#)]
            ),
            "missing {source}"
        );
    }
}

async fn read_command<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Option<Vec<String>> {
    let mut line = String::new();
    reader.read_line(&mut line).await.ok()?;
    let count: usize = line.trim().strip_prefix('*')?.parse().ok()?;
    let mut args = Vec::with_capacity(count);
    for _ in 0..count {
        line.clear();
        reader.read_line(&mut line).await.ok()?;
        let len: usize = line.trim().strip_prefix('$')?.parse().ok()?;
        let mut bytes = vec![0; len + 2];
        reader.read_exact(&mut bytes).await.ok()?;
        args.push(String::from_utf8_lossy(&bytes[..len]).into_owned());
    }
    Some(args)
}

async fn silent_after_ping() -> RedisUrl {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = RedisUrl::parse(&format!("redis://{}", listener.local_addr().unwrap())).unwrap();
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (read, mut write) = socket.into_split();
                let mut reader = BufReader::new(read);
                while let Some(args) = read_command(&mut reader).await {
                    let reply: &[u8] = match args.first().map(|a| a.to_ascii_uppercase()) {
                        Some(cmd) if cmd == "PING" => b"+PONG\r\n",
                        Some(cmd) if cmd == "CLIENT" => b"+OK\r\n",
                        _ => {
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            return;
                        }
                    };
                    if write.write_all(reply).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    url
}

#[tokio::test]
async fn redis_failures_are_recorded() {
    let handle = handle();
    let secret = KeySecret::parse("made-up-secret-made-up-secret-0001").unwrap();
    let shared = SharedCache::connect(&silent_after_ping().await, secret)
        .await
        .unwrap();

    assert_eq!(shared.get::<u32>("usm:v1:test:metrics").await, None);

    let rendered = handle.render();
    assert!(rendered.lines().any(|l| l == "redis_up 0"));
    assert!(has(&rendered, "redis_errors_total", &["reason="]));
    assert!(has(
        &rendered,
        "redis_op_duration_seconds_bucket",
        &[r#"op="get""#]
    ));
}

#[tokio::test]
async fn searches_record_how_deep_they_go() {
    let handle = handle();
    let server = MockServer::start().await;
    let fans: Vec<i64> = (0..10).map(|day| 100_000_000 + 4_000_000 * day).collect();
    let members: Vec<_> = (0..10)
        .map(|i| json!({ "viewer_id": 7000 + i, "year": 2026, "month": 3, "daily_fans": fans }))
        .collect();
    let circle = json!({
        "circle_id": 900000401,
        "name": "Depth Club",
        "member_count": 10,
        "join_style": 1,
        "monthly_rank": 6_000,
        "monthly_point": 20_000_000,
        "last_updated": "2026-03-19T15:00:00Z",
    });
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": [circle] })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000401"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "circle": circle, "members": members })),
        )
        .mount(&server)
        .await;
    let client = client(server.uri());
    let details = ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0;
    let config = ScanConfig {
        rank_from: 1,
        rank_to: 10_000,
        ..ScanConfig::default()
    };
    scan(&client, &details, &config).await.unwrap();
    let state = AppState {
        details,
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
        ..state()
    };

    let status = get(app(state, None), "/api/v1/recommendations?pace=4000000").await;
    assert_eq!(status, StatusCode::OK);
    let rendered = handle.render();
    for name in ["search_shortlist_worst_rank", "search_pick_worst_rank"] {
        assert!(
            has(&rendered, &format!("{name}_bucket"), &[r#"le="7500""#]),
            "missing {name}"
        );
    }
    assert!(rendered
        .lines()
        .any(|l| l.starts_with("search_pool_bottom_total ")));
}
