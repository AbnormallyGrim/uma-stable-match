use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uma_stable_match_server::{router, run_scheduler, Cache, CacheStatus, ScanConfig};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::path;
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

fn list() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circles": [{ "circle_id": 1, "name": "Schedule Club", "monthly_rank": 100, "member_count": 20 }],
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
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

fn start(server: &MockServer, interval: Duration) -> Running {
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server),
        ScanConfig::default(),
        interval,
        cache.clone(),
        stopped,
    ));
    Running {
        cache,
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

async fn status_json(cache: &Cache) -> Value {
    let response = router(cache.clone())
        .oneshot(Request::get("/api/status").body(Body::empty()).unwrap())
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
    let status = status_json(&Cache::new()).await;
    assert_eq!(
        status,
        json!({ "scanning": false, "last_scan": null, "last_error": null })
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

    let status = status_json(&running.cache).await;
    assert_eq!(status["scanning"], false);
    assert_eq!(status["last_scan"]["clubs"], 1);
    assert_eq!(status["last_scan"]["candidates"], 1);
    assert_eq!(status["last_scan"]["failed"], 0);
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

    let status = status_json(&running.cache).await;
    assert_eq!(status["last_scan"]["clubs"], 1);
    assert!(status["last_error"]["message"]
        .as_str()
        .unwrap()
        .contains("503"));
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
