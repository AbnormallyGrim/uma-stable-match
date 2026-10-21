//! Scheduler and status route tests against a mock server. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uma_stable_match_server::{
    router, run_scheduler, AppState, Cache, CacheStatus, ClubDetails, DetailConfig, MatchSettings,
    PaceLookup, RateLimits, RefreshConfig, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn offline() -> Client {
    Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn details(client: &Client) -> ClubDetails {
    ClubDetails::new(client, SharedPool::default(), DetailConfig::default()).0
}

fn app(cache: Cache, details: ClubDetails) -> AppState {
    AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(offline(), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

fn list() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circles": [{
            "circle_id": 1,
            "name": "Schedule Club",
            "monthly_rank": 100,
            "member_count": 20,
            "monthly_point": 20_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        }],
    }))
}

fn detail() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": { "circle_id": 1, "name": "Schedule Club", "member_count": 20 },
        "members": [],
    }))
}

async fn mount_detail(server: &MockServer) {
    Mock::given(path("/api/v4/circles"))
        .respond_with(detail())
        .mount(server)
        .await;
}

struct Running {
    cache: Cache,
    details: ClubDetails,
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

fn start(server: &MockServer, interval: Duration) -> Running {
    start_with(server, details(&client(server)), Some(interval))
}

fn start_with(server: &MockServer, details: ClubDetails, every: Option<Duration>) -> Running {
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server),
        details.clone(),
        ScanConfig::default(),
        every,
        cache.clone(),
        stopped,
    ));
    Running {
        cache,
        details,
        stop,
        handle,
    }
}

async fn wait_until(cache: &Cache, done: impl Fn(&CacheStatus) -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !done(&cache.status()) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition not reached in time");
}

async fn stop(running: Running) {
    running.stop.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(1), running.handle)
        .await
        .expect("scheduler did not stop")
        .unwrap();
}

async fn status_json(cache: &Cache, details: &ClubDetails) -> Value {
    let response = router(app(cache.clone(), details.clone()))
        .oneshot(Request::get("/api/v1/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn list_calls(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == "/api/v4/circles/list")
        .count()
}

#[tokio::test]
async fn status_before_first_scan() {
    let status = status_json(&Cache::new(), &details(&offline())).await;
    assert_eq!(
        status,
        json!({
            "scanning": false,
            "scan": null,
            "last_scan": null,
            "last_error": null,
            "pool": { "clubs": 0, "exact_medians": 0, "calibration": { "factor": 1.0, "samples": 0 } },
            "details": { "cached": 0, "queued": 0 },
        })
    );
}

#[tokio::test]
async fn scans_at_startup() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(list())
        .mount(&server)
        .await;
    mount_detail(&server).await;

    let running = start(&server, Duration::from_secs(3600));
    wait_until(&running.cache, |s| s.snapshot.is_some()).await;

    let status = status_json(&running.cache, &running.details).await;
    assert_eq!(status["scanning"], false);
    assert_eq!(status["last_scan"]["clubs"], 1);
    assert_eq!(status["last_scan"]["candidates"], 1);
    assert_eq!(status["last_scan"]["failed"], 0);
    assert_eq!(status["last_scan"]["month"], "2026-03");
    assert_eq!(status["pool"]["clubs"], 1);
    assert!(status["last_scan"]["finished_at"]
        .as_str()
        .unwrap()
        .ends_with('Z'));
    assert_eq!(status["last_error"], Value::Null);

    stop(running).await;
    assert_eq!(list_calls(&server).await, 1);
}

#[tokio::test]
async fn rescans_on_the_interval() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(list())
        .mount(&server)
        .await;
    mount_detail(&server).await;

    let running = start(&server, Duration::from_millis(50));
    tokio::time::timeout(Duration::from_secs(5), async {
        while list_calls(&server).await < 3 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("expected repeated scans");
    stop(running).await;
}

fn refreshing(server: &MockServer) -> ClubDetails {
    let config = DetailConfig {
        refresh: RefreshConfig::default(),
        refresher: true,
        ..DetailConfig::default()
    };
    ClubDetails::new(&client(server), SharedPool::default(), config).0
}

async fn mount_list(server: &MockServer) {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(list())
        .mount(server)
        .await;
}

#[tokio::test]
async fn with_a_refresher_lists_once_until_uma_moes_next_update() {
    let server = MockServer::start().await;
    mount_list(&server).await;
    let running = start_with(&server, refreshing(&server), None);
    wait_until(&running.cache, |s| s.snapshot.is_some()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(list_calls(&server).await, 1);
    assert_eq!(running.details.scheduled().await, 1);
    stop(running).await;
}

#[tokio::test]
async fn with_a_refresher_a_set_interval_overrides_the_daily_update() {
    let server = MockServer::start().await;
    mount_list(&server).await;
    let running = start_with(
        &server,
        refreshing(&server),
        Some(Duration::from_millis(50)),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while list_calls(&server).await < 3 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("expected repeated scans");
    stop(running).await;
}

#[tokio::test]
async fn without_a_refresher_or_interval_scans_on_the_default_interval() {
    let server = MockServer::start().await;
    mount_list(&server).await;
    mount_detail(&server).await;
    let running = start_with(&server, details(&client(&server)), None);
    wait_until(&running.cache, |s| s.snapshot.is_some()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(list_calls(&server).await, 1);
    stop(running).await;
}

#[tokio::test]
async fn failed_scan_keeps_last_results() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(list())
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    mount_detail(&server).await;

    let running = start(&server, Duration::from_millis(50));
    wait_until(&running.cache, |s| s.last_error.is_some()).await;

    let status = status_json(&running.cache, &running.details).await;
    assert_eq!(status["last_scan"]["clubs"], 1);
    assert_eq!(
        status["last_error"]["message"],
        "listing clubs failed: uma.moe returned 503 Service Unavailable"
    );
    stop(running).await;
}

#[tokio::test]
async fn shutdown_cancels_a_running_scan() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(list().set_delay(Duration::from_secs(10)))
        .mount(&server)
        .await;

    let running = start(&server, Duration::from_secs(3600));
    wait_until(&running.cache, |s| s.scanning).await;
    let cache = running.cache.clone();
    stop(running).await;

    let status = cache.status();
    assert!(!status.scanning);
    assert_eq!(status.snapshot, None);
}

#[tokio::test]
async fn unreachable_api_error_hides_the_url() {
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let unreachable = Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    let unreachable_details = details(&unreachable);
    let handle = tokio::spawn(run_scheduler(
        unreachable,
        unreachable_details.clone(),
        ScanConfig::default(),
        Some(Duration::from_secs(3600)),
        cache.clone(),
        stopped,
    ));
    wait_until(&cache, |s| s.last_error.is_some()).await;
    stop.send(true).unwrap();
    handle.await.unwrap();

    let status = status_json(&cache, &unreachable_details).await;
    let message = status["last_error"]["message"].as_str().unwrap();
    assert_eq!(message, "listing clubs failed: could not reach uma.moe");
    assert!(!message.contains("http"));
    assert!(!message.contains("127.0.0.1"));
}

#[tokio::test]
async fn status_reports_the_running_scan() {
    let server = MockServer::start().await;
    let circles: Vec<Value> = [1, 2]
        .iter()
        .map(|id| {
            json!({
                "circle_id": id,
                "name": "Progress Club",
                "monthly_rank": 100 + id,
                "member_count": 20,
                "monthly_point": 20_000_000 * id,
                "last_updated": "2026-03-19T15:00:00Z",
            })
        })
        .collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": circles })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(detail())
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "2"))
        .respond_with(detail().set_delay(Duration::from_secs(30)))
        .mount(&server)
        .await;

    let running = start(&server, Duration::from_secs(3600));
    wait_until(&running.cache, |s| {
        s.progress.as_ref().is_some_and(|p| p.loaded == 1)
    })
    .await;

    let status = status_json(&running.cache, &running.details).await;
    assert_eq!(status["scanning"], true);
    assert_eq!(status["scan"]["phase"], "loading");
    assert_eq!(status["scan"]["listed"], 2);
    assert_eq!(status["scan"]["to_load"], 2);
    assert_eq!(status["scan"]["loaded"], 1);
    assert_eq!(status["scan"]["failed"], 0);
    assert!(status["scan"]["started_at"]
        .as_str()
        .unwrap()
        .ends_with('Z'));

    let cache = running.cache.clone();
    stop(running).await;
    assert_eq!(cache.status().progress, None);
}
