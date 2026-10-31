//! Redis shared-cache tests. All data is made up.
//!
//! The tests marked as needing Redis run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it. The others use a fake server.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uma_stable_match_server::{
    club_key, install_metrics, metrics_router, router, run_scheduler, scan, AppState, Cache,
    CacheStatus, ClubDetails, ClubPool, DetailConfig, KeySecret, ListedClub, MatchSettings,
    PaceLookup, RateLimits, RedisUrl, RefreshConfig, ScanConfig, ScanPhase, SharedCache,
    SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, Priority, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn secret() -> KeySecret {
    KeySecret::parse("made-up-secret-made-up-secret-0001").unwrap()
}

async fn redis() -> Option<SharedCache> {
    let Some(url) = std::env::var("REDIS_TEST_URL")
        .ok()
        .and_then(|u| RedisUrl::parse(&u))
    else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    Some(
        SharedCache::connect(&url, secret())
            .await
            .expect("REDIS_TEST_URL is reachable"),
    )
}

fn unique_id() -> u64 {
    fastrand::u64(100_000_000_000..900_000_000_000)
}

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

fn pool(id: u64) -> SharedPool {
    Arc::new(RwLock::new(ClubPool::new([ListedClub {
        circle_id: id,
        member_count: 9,
        join_style: None,
        raw_rate: 1000.0,
        policy: None,
        discord: false,
    }])))
}

fn detail(id: u64) -> serde_json::Value {
    let members: Vec<_> = [100, 200, 300, 400, 500, 600, 700, 800, 5000]
        .iter()
        .enumerate()
        .map(|(i, gain)| {
            let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + gain * day).collect();
            json!({ "viewer_id": 7001 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    json!({ "circle": { "circle_id": id, "name": "Shared Club", "member_count": 9 }, "members": members })
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
async fn unreachable_redis_fails_at_start() {
    let url = RedisUrl::parse("redis://user:made-up-password@127.0.0.1:1").unwrap();
    let started = Instant::now();
    let error = SharedCache::connect(&url, secret()).await.unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(error.to_string().starts_with("could not connect to Redis"));
    assert!(!error.to_string().contains("made-up-password"));
}

#[tokio::test]
async fn a_redis_that_stops_answering_counts_as_a_miss() {
    let shared = SharedCache::connect(&silent_after_ping().await, secret())
        .await
        .unwrap();
    let started = Instant::now();
    assert_eq!(shared.get::<u32>("usm:v1:test:silent").await, None);
    shared
        .set("usm:v1:test:silent", &1u32, Duration::from_secs(60))
        .await;
    let keys = [
        "usm:v1:test:silent".to_owned(),
        "usm:v1:test:quiet".to_owned(),
    ];
    assert_eq!(shared.get_many::<u32>(&keys).await, vec![None, None]);
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn requests_still_work_when_redis_stops_answering() {
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(id)))
        .expect(1)
        .mount(&server)
        .await;
    let shared = SharedCache::connect(&silent_after_ping().await, secret())
        .await
        .unwrap();
    let (details, _loader) =
        ClubDetails::with_shared(client(&server), pool(id), DetailConfig::default(), shared);

    let club = details.get(id, Priority::Interactive).await.unwrap();
    assert_eq!(club.circle_id, id);
}

#[tokio::test]
async fn values_round_trip_and_expire() {
    let Some(shared) = redis().await else { return };
    assert!(shared.is_enabled());
    let key = format!("usm:v1:test:{}", unique_id());
    shared
        .set(&key, &vec![1u32, 2, 3], Duration::from_millis(400))
        .await;
    assert_eq!(shared.get::<Vec<u32>>(&key).await, Some(vec![1, 2, 3]));
    assert_eq!(shared.get::<String>(&key).await, None);
    let missing = format!("usm:v1:test:{}", unique_id());
    assert_eq!(
        shared
            .get_many::<Vec<u32>>(&[missing.clone(), key.clone(), missing])
            .await,
        vec![None, Some(vec![1, 2, 3]), None]
    );
    assert_eq!(
        shared.get_many::<Vec<u32>>(&[]).await,
        Vec::<Option<Vec<u32>>>::new()
    );
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(shared.get::<Vec<u32>>(&key).await, None);
}

#[tokio::test]
async fn instances_share_club_details() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let second_shared = redis().await.unwrap();
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(id)))
        .expect(1)
        .mount(&server)
        .await;

    let config = DetailConfig::default();
    let (first, _a) = ClubDetails::with_shared(client(&server), pool(id), config, first_shared);
    let (second, _b) =
        ClubDetails::with_shared(client(&server), pool(id), config, second_shared.clone());

    first.get(id, Priority::Interactive).await.unwrap();
    let club = second.get(id, Priority::Interactive).await.unwrap();
    assert_eq!(club.name, "Shared Club");
    assert_eq!(
        second.pool().read().unwrap().estimated_median(id),
        Some(500.0)
    );
    assert!(second_shared
        .get::<serde_json::Value>(&club_key(id, None))
        .await
        .is_some());
}

#[tokio::test]
async fn the_club_page_serves_a_club_another_instance_loaded() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let second_shared = redis().await.unwrap();
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(id)))
        .expect(1)
        .mount(&server)
        .await;

    let config = DetailConfig::default();
    let (first, _a) = ClubDetails::with_shared(client(&server), pool(id), config, first_shared);
    let (second, _b) = ClubDetails::with_shared(client(&server), pool(id), config, second_shared);
    first.get(id, Priority::Interactive).await.unwrap();

    let response = router(AppState {
        cache: Cache::new(),
        details: second,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client(&server), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    })
    .oneshot(
        Request::get(format!("/api/v1/clubs/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["club"]["name"], "Shared Club");
    assert_eq!(body["members"].as_array().unwrap().len(), 9);
}

#[tokio::test]
async fn instances_share_trainer_paces_under_hashed_keys() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let second_shared = redis().await.unwrap();
    let server = MockServer::start().await;
    let trainer = unique_id();
    let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + 3_000_000 * day).collect();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", trainer.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": { "circle_id": 2 },
            "members": [{ "viewer_id": trainer, "year": 2026, "month": 3, "daily_fans": fans }],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let first = PaceLookup::new(client(&server), DEFAULT_PACE_TTL).with_shared(first_shared);
    let second =
        PaceLookup::new(client(&server), DEFAULT_PACE_TTL).with_shared(second_shared.clone());
    assert_eq!(first.pace(trainer, None).await.unwrap().pace, 3_000_000.0);
    assert_eq!(second.pace(trainer, None).await.unwrap().pace, 3_000_000.0);

    let key = second_shared.pace_key(trainer, None);
    assert!(!key.contains(&trainer.to_string()));
    assert!(second_shared.get::<serde_json::Value>(&key).await.is_some());

    let url = std::env::var("REDIS_TEST_URL").unwrap();
    let mut conn = redis::Client::open(url)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("usm:v1:pace:*")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!(keys.iter().all(|k| !k.contains(&trainer.to_string())));
}

fn scan_config() -> ScanConfig {
    ScanConfig {
        rank_from: 1,
        rank_to: fastrand::u32(1_000..1_000_000),
        ..ScanConfig::default()
    }
}

async fn mount_scan(server: &MockServer, id: u64) {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_club(id))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(id)))
        .expect(1)
        .mount(server)
        .await;
}

fn listed_club(id: u64) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circles": [{
            "circle_id": id,
            "name": "Shared Club",
            "monthly_rank": 100,
            "member_count": 9,
            "monthly_point": 20_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        }],
    }))
}

struct Instance {
    cache: Cache,
    details: ClubDetails,
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

fn instance(server: &MockServer, config: ScanConfig, shared: SharedCache) -> Instance {
    let cache = Cache::new();
    let (details, _loader) = ClubDetails::with_shared(
        client(server),
        SharedPool::default(),
        DetailConfig::default(),
        shared,
    );
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server),
        details.clone(),
        config,
        Some(Duration::from_secs(3600)),
        cache.clone(),
        stopped,
    ));
    Instance {
        cache,
        details,
        stop,
        handle,
    }
}

impl Instance {
    async fn wait_until(&self, done: impl Fn(&CacheStatus) -> bool) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while !done(&self.cache.status()) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("condition not reached in time");
    }

    async fn stop(self) {
        self.stop.send(true).unwrap();
        tokio::time::timeout(Duration::from_secs(2), self.handle)
            .await
            .expect("scheduler did not stop")
            .unwrap();
    }
}

#[tokio::test]
async fn scans_alone_when_redis_stops_answering() {
    let server = MockServer::start().await;
    mount_scan(&server, unique_id()).await;
    let shared = SharedCache::connect(&silent_after_ping().await, secret())
        .await
        .unwrap();

    let only = instance(&server, scan_config(), shared);
    only.wait_until(|s| s.snapshot.is_some()).await;
    assert_eq!(only.cache.snapshot().unwrap().candidates, 1);
    only.stop().await;
}

#[tokio::test]
async fn instances_share_one_scan() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let second_shared = redis().await.unwrap();
    let server = MockServer::start().await;
    let id = unique_id();
    mount_scan(&server, id).await;
    let config = scan_config();

    let first = instance(&server, config, first_shared);
    let second = instance(&server, config, second_shared);
    for running in [&first, &second] {
        running.wait_until(|s| s.snapshot.is_some()).await;
        let snapshot = running.cache.snapshot().unwrap();
        assert_eq!(snapshot.candidates, 1);
        assert_eq!(snapshot.prefetched, 1);
        assert_eq!(snapshot.month.as_deref(), Some("2026-03"));
        assert_eq!(
            running.details.pool().read().unwrap().estimated_median(id),
            Some(500.0)
        );
    }

    let late = instance(&server, config, redis().await.unwrap());
    late.wait_until(|s| s.snapshot.is_some()).await;
    assert_eq!(late.cache.snapshot().unwrap().candidates, 1);

    for running in [first, second, late] {
        running.stop().await;
    }
}

#[tokio::test]
async fn a_failed_shared_scan_is_reported_by_every_instance() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let second_shared = redis().await.unwrap();
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let config = scan_config();

    let first = instance(&server, config, first_shared);
    let second = instance(&server, config, second_shared);
    for running in [&first, &second] {
        running.wait_until(|s| s.last_error.is_some()).await;
        assert_eq!(
            running.cache.status().last_error.unwrap().message,
            "listing clubs failed: uma.moe returned 503 Service Unavailable"
        );
    }

    for running in [first, second] {
        running.stop().await;
    }
}

fn listed_clubs(clubs: &[(u64, i64)]) -> ResponseTemplate {
    listed_clubs_at(clubs, "2026-03-19T15:00:00Z")
}

fn listed_clubs_at(clubs: &[(u64, i64)], last_updated: &str) -> ResponseTemplate {
    let circles: Vec<_> = clubs
        .iter()
        .enumerate()
        .map(|(i, (id, points))| {
            json!({
                "circle_id": id,
                "name": "Resumed Club",
                "monthly_rank": 100 + i,
                "member_count": 9,
                "monthly_point": points,
                "last_updated": last_updated,
            })
        })
        .collect();
    ResponseTemplate::new(200).set_body_json(json!({ "circles": circles }))
}

async fn mount_slow_scan(server: &MockServer, fast: u64, slow: u64, delay: Duration) {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[(fast, 10_000_000), (slow, 20_000_000)]))
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", fast.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(fast)))
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", slow.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(detail(slow))
                .set_delay(delay),
        )
        .mount(server)
        .await;
}

async fn calls(server: &MockServer, path: &str, query: Option<String>) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == path && (query.is_none() || r.url.query() == query.as_deref()))
        .count()
}

async fn wait_for(done: impl AsyncFn() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !done().await {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("condition not reached in time");
}

async fn key_exists(key: &str) -> bool {
    let url = std::env::var("REDIS_TEST_URL").unwrap();
    let mut conn = redis::Client::open(url)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    redis::cmd("EXISTS")
        .arg(key)
        .query_async::<i64>(&mut conn)
        .await
        .unwrap()
        == 1
}

#[tokio::test]
async fn a_restarted_leader_resumes_the_scan_without_listing_again() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let (fast, slow) = (unique_id(), unique_id());
    mount_slow_scan(&server, fast, slow, Duration::from_secs(2)).await;
    let config = scan_config();
    let progress = format!(
        "usm:v1:scan:{}-{}-{}:progress",
        config.rank_from, config.rank_to, config.max_details
    );

    let before_first = SystemTime::now();
    let first = instance(&server, config, first_shared);
    wait_for(async || key_exists(&club_key(fast, None)).await).await;
    assert!(key_exists(&progress).await);
    first.stop().await;

    let before_second = SystemTime::now();
    let second = instance(&server, config, redis().await.unwrap());
    second
        .wait_until(|s| {
            s.progress
                .as_ref()
                .is_some_and(|p| p.phase == ScanPhase::Loading)
        })
        .await;
    let started = second.cache.status().progress.unwrap().started_at.unwrap();
    assert!(before_first - Duration::from_millis(1) <= started && started < before_second);
    second.wait_until(|s| s.snapshot.is_some()).await;
    let snapshot = second.cache.snapshot().unwrap();
    assert_eq!(snapshot.candidates, 2);
    assert_eq!(snapshot.prefetched, 2);

    assert_eq!(calls(&server, "/api/v4/circles/list", None).await, 1);
    assert_eq!(
        calls(
            &server,
            "/api/v4/circles",
            Some(format!("circle_id={fast}"))
        )
        .await,
        1
    );
    assert!(!key_exists(&progress).await);
    second.stop().await;
}

#[tokio::test]
async fn waiting_instances_build_their_pool_from_the_scan_in_progress() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let (fast, slow) = (unique_id(), unique_id());
    mount_slow_scan(&server, fast, slow, Duration::from_secs(5)).await;
    let config = scan_config();

    let first = instance(&server, config, first_shared);
    wait_for(async || calls(&server, "/api/v4/circles/list", None).await == 1).await;
    let second = instance(&server, config, redis().await.unwrap());
    wait_for(async || second.details.pool().read().unwrap().len() == 2).await;

    assert!(first.cache.snapshot().is_none());
    assert!(second.cache.snapshot().is_none());
    assert!(second.cache.status().scanning);
    let waiting = second.cache.status().progress.unwrap();
    assert_eq!(waiting.phase, ScanPhase::Waiting);
    assert_eq!(waiting.listed, 2);
    let leader_started = first.cache.status().progress.unwrap().started_at.unwrap();
    let waiting_started = waiting.started_at.unwrap();
    assert!(
        leader_started
            .duration_since(waiting_started)
            .unwrap_or_default()
            < Duration::from_millis(1)
    );
    assert_eq!(calls(&server, "/api/v4/circles/list", None).await, 1);

    for running in [first, second] {
        running.stop().await;
    }
}

#[tokio::test]
async fn a_refresh_reaches_other_instances_with_its_update_time() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    let respond = |name: &str| {
        let mut body = detail(id);
        body["circle"]["name"] = json!(name);
        body["circle"]["last_updated"] = json!("2026-03-19T15:00:00Z");
        ResponseTemplate::new(200).set_body_json(body)
    };
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(respond("Before"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(respond("After"))
        .mount(&server)
        .await;
    let config = DetailConfig::default();
    let (first, _a) = ClubDetails::with_shared(client(&server), pool(id), config, first_shared);
    let (second, _b) =
        ClubDetails::with_shared(client(&server), pool(id), config, redis().await.unwrap());

    first.get(id, Priority::Interactive).await.unwrap();
    first.refresh(id, Priority::Background).await.unwrap();
    let club = second.get(id, Priority::Interactive).await.unwrap();
    assert_eq!(club.name, "After");
    assert_eq!(
        second.last_updated(id).await,
        Some("2026-03-19T15:00:00Z".parse().unwrap())
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

async fn ttl_millis(key: &str) -> i64 {
    let url = std::env::var("REDIS_TEST_URL").unwrap();
    let mut conn = redis::Client::open(url)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    redis::cmd("PTTL")
        .arg(key)
        .query_async::<i64>(&mut conn)
        .await
        .unwrap()
}

fn updated(id: u64, name: &str, last_updated: &str) -> ResponseTemplate {
    let mut body = detail(id);
    body["circle"]["name"] = json!(name);
    body["circle"]["last_updated"] = json!(last_updated);
    ResponseTemplate::new(200).set_body_json(body)
}

fn every(millis: u64) -> DetailConfig {
    DetailConfig {
        refresh: RefreshConfig {
            every: Some(Duration::from_millis(millis)),
            ..RefreshConfig::default()
        },
        ..DetailConfig::default()
    }
}

#[tokio::test]
async fn club_details_stay_in_redis_a_day_past_their_next_refresh() {
    let Some(shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let last_updated = (now - Duration::from_secs(3600)).to_rfc3339();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(updated(id, "Kept", &last_updated))
        .mount(&server)
        .await;
    let (details, _) =
        ClubDetails::with_shared(client(&server), pool(id), DetailConfig::default(), shared);

    details.get(id, Priority::Interactive).await.unwrap();
    let ttl = Duration::from_millis(ttl_millis(&club_key(id, None)).await as u64);
    let expected = Duration::from_secs(23 * 3600 + 30 * 60 + 24 * 3600);
    assert!(ttl <= expected, "{ttl:?}");
    assert!(ttl > expected - Duration::from_secs(60), "{ttl:?}");
}

#[tokio::test]
async fn a_club_past_its_next_refresh_is_fetched_again_through_redis() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(updated(id, "Before", "2026-03-19T15:00:00Z"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(updated(id, "After", "2026-03-20T15:00:00Z"))
        .mount(&server)
        .await;
    let (first, _a) = ClubDetails::with_shared(client(&server), pool(id), every(100), first_shared);
    let (second, _b) = ClubDetails::with_shared(
        client(&server),
        pool(id),
        every(100),
        redis().await.unwrap(),
    );

    assert_eq!(
        first.get(id, Priority::Interactive).await.unwrap().name,
        "Before"
    );
    assert_eq!(
        second.get(id, Priority::Interactive).await.unwrap().name,
        "Before"
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        second.get(id, Priority::Interactive).await.unwrap().name,
        "After"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn the_last_copy_in_redis_is_served_when_uma_moe_fails() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(updated(id, "Kept", "2026-03-19T15:00:00Z"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let (first, _a) = ClubDetails::with_shared(client(&server), pool(id), every(50), first_shared);
    let (second, _b) =
        ClubDetails::with_shared(client(&server), pool(id), every(50), redis().await.unwrap());

    first.get(id, Priority::Interactive).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        second.get(id, Priority::Interactive).await.unwrap().name,
        "Kept"
    );
    assert_eq!(
        second.get(id, Priority::Interactive).await.unwrap().name,
        "Kept"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

fn refreshing(server: &MockServer, scan: ScanConfig, shared: SharedCache) -> ClubDetails {
    let config = DetailConfig {
        refresher: true,
        scan,
        ..DetailConfig::default()
    };
    ClubDetails::with_shared(client(server), SharedPool::default(), config, shared).0
}

async fn mount_details(server: &MockServer, ids: &[u64]) {
    for id in ids {
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(detail(*id)))
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn the_refresh_schedule_is_shared_by_every_instance() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id(), unique_id()];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[
            (ids[0], 10_000_000),
            (ids[1], 20_000_000),
            (ids[2], 30_000_000),
        ]))
        .mount(&server)
        .await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());

    scan(&client(&server), &first, &config).await.unwrap();
    assert_eq!(second.scheduled().await, 3);
    for id in ids {
        assert!(second.next_refresh(id).await.is_some());
    }
}

#[tokio::test]
async fn only_one_instance_refreshes_each_club() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id(), unique_id()];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[
            (ids[0], 10_000_000),
            (ids[1], 20_000_000),
            (ids[2], 30_000_000),
        ]))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();

    let a = tokio::spawn(first.refresher().unwrap().run());
    let b = tokio::spawn(second.refresher().unwrap().run());
    wait_for(async || calls(&server, "/api/v4/circles", None).await == 3).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    for id in ids {
        assert_eq!(
            calls(&server, "/api/v4/circles", Some(format!("circle_id={id}"))).await,
            1
        );
        let due = first.next_refresh(id).await.unwrap();
        assert!(due > chrono::DateTime::<chrono::Utc>::from(SystemTime::now()));
    }
    a.abort();
    b.abort();
}

#[tokio::test]
async fn clubs_loaded_by_another_instance_join_the_shared_schedule() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id()];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[(ids[0], 10_000_000), (ids[1], 20_000_000)]))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = ScanConfig {
        max_details: 1,
        ..scan_config()
    };
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();
    scan(&client(&server), &second, &config).await.unwrap();
    assert_eq!(first.scheduled().await, 1);

    let unscheduled = if first.next_refresh(ids[0]).await.is_some() {
        ids[1]
    } else {
        ids[0]
    };
    second
        .get(unscheduled, Priority::Interactive)
        .await
        .unwrap();
    assert!(first.next_refresh(unscheduled).await.is_some());
    assert_eq!(first.scheduled().await, 2);
}

#[tokio::test]
async fn the_refresher_skips_clubs_another_instance_already_loaded() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[(id, 10_000_000)]))
        .mount(&server)
        .await;
    mount_details(&server, &[id]).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();
    scan(&client(&server), &second, &config).await.unwrap();
    second.get(id, Priority::Interactive).await.unwrap();

    let refresher = tokio::spawn(first.refresher().unwrap().run());
    let loaded = first.next_refresh(id).await;
    wait_for(async || first.next_refresh(id).await != loaded).await;
    assert_eq!(calls(&server, "/api/v4/circles", None).await, 1);
    refresher.abort();
}

#[tokio::test]
async fn a_shared_scan_with_a_refresher_lasts_until_uma_moes_next_update() {
    let Some(shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let id = unique_id();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_club(id))
        .mount(&server)
        .await;
    let config = scan_config();
    let cache = Cache::new();
    let details = refreshing(&server, config, shared);
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(&server),
        details.clone(),
        config,
        None,
        cache.clone(),
        stopped,
    ));
    tokio::time::timeout(Duration::from_secs(10), async {
        while cache.snapshot().is_none() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();

    let fresh = format!(
        "usm:v1:scan:{}-{}-{}:fresh",
        config.rank_from, config.rank_to, config.max_details
    );
    let ttl = Duration::from_millis(ttl_millis(&fresh).await as u64);
    assert!(
        ttl > Duration::from_secs(3500) && ttl <= Duration::from_secs(3600),
        "{ttl:?}"
    );
    assert_eq!(details.scheduled().await, 1);
    stop.send(true).unwrap();
    handle.await.unwrap();
}

#[tokio::test]
async fn coverage_counts_scheduled_clubs_stored_in_redis() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id(), unique_id()];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[
            (ids[0], 10_000_000),
            (ids[1], 20_000_000),
            (ids[2], 30_000_000),
        ]))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();
    first.get(ids[0], Priority::Interactive).await.unwrap();
    first.get(ids[1], Priority::Interactive).await.unwrap();

    let status = second.refresh_status().await;
    assert_eq!((status.scheduled, status.loaded, status.due), (3, 2, 3));
    let ttl = status.stored_ttl.expect("stored clubs have a TTL");
    assert!(ttl.shortest > Duration::ZERO && ttl.shortest <= ttl.average);

    let url = std::env::var("REDIS_TEST_URL").unwrap();
    let mut conn = redis::Client::open(url)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    redis::cmd("DEL")
        .arg(club_key(ids[0], None))
        .query_async::<i64>(&mut conn)
        .await
        .unwrap();
    assert_eq!(second.refresh_status().await.loaded, 1);
}

#[tokio::test]
async fn a_restarted_instance_scores_clubs_from_redis_without_fetching_them() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id(), unique_id()];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&[
            (ids[0], 10_000_000),
            (ids[1], 20_000_000),
            (ids[2], 30_000_000),
        ]))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    scan(&client(&server), &first, &config).await.unwrap();
    for id in ids {
        first.get(id, Priority::Interactive).await.unwrap();
    }

    let restarted = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &restarted, &config).await.unwrap();
    assert_eq!(restarted.cached_clubs(), 0);
    let state = AppState {
        cache: Cache::new(),
        details: restarted.clone(),
        matching: MatchSettings {
            shortlist_size: 3,
            fetch_budget: 0,
        },
        paces: PaceLookup::new(client(&server), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
        human: Default::default(),
    };
    let response = router(state)
        .oneshot(
            Request::get("/api/v1/recommendations?pace=500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();

    assert_eq!(
        body["coverage"],
        json!({ "considered": 3, "shortlisted": 3, "scored": 3, "pending": 0 })
    );
    assert_eq!(restarted.queued(), 0);
    assert_eq!(calls(&server, "/api/v4/circles", None).await, 3);
}

// Loads three clubs on one instance, then scans on a restarted one whose listing was updated at
// `relisted_at`.
async fn restarted_after_loading(relisted_at: &str) -> Option<(ClubDetails, [u64; 3])> {
    let first_shared = redis().await?;
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id(), unique_id()];
    let clubs = [
        (ids[0], 10_000_000),
        (ids[1], 20_000_000),
        (ids[2], 30_000_000),
    ];
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs(&clubs))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(listed_clubs_at(&clubs, relisted_at))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    scan(&client(&server), &first, &config).await.unwrap();
    for id in ids {
        first.get(id, Priority::Interactive).await.unwrap();
    }

    let restarted = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &restarted, &config).await.unwrap();
    assert_eq!(restarted.cached_clubs(), 0);
    assert_eq!(calls(&server, "/api/v4/circles", None).await, 3);
    Some((restarted, ids))
}

#[tokio::test]
async fn a_restarted_instance_starts_with_the_exact_medians_in_redis() {
    let Some((restarted, ids)) = restarted_after_loading("2026-03-20T15:00:00Z").await else {
        return;
    };
    let pool = restarted.pool().read().unwrap();
    assert_eq!(pool.exact_medians(), 3);
    assert_eq!(pool.estimated_median(ids[0]), Some(500.0));
}

#[tokio::test]
async fn a_restarted_instance_ignores_another_months_medians() {
    let Some((restarted, _)) = restarted_after_loading("2026-04-19T15:00:00Z").await else {
        return;
    };
    assert_eq!(restarted.pool().read().unwrap().exact_medians(), 0);
}

#[tokio::test]
async fn full_clubs_have_their_own_shared_schedule() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id()];
    let circles: Vec<_> = ids
        .iter()
        .zip([9, 30])
        .enumerate()
        .map(|(i, (id, members))| {
            json!({
                "circle_id": id,
                "name": "Tiered Club",
                "monthly_rank": 100 + i,
                "member_count": members,
                "monthly_point": 20_000_000,
                "last_updated": "2026-03-19T15:00:00Z",
            })
        })
        .collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": circles })))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let config = scan_config();
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();
    first.get(ids[1], Priority::Interactive).await.unwrap();

    let status = second.refresh_status().await;
    assert_eq!(status.scheduled, 1);
    assert_eq!((status.full.scheduled, status.full.loaded), (1, 1));
    assert!(second.next_refresh(ids[1]).await.is_some());
}

#[tokio::test]
async fn lower_range_clubs_have_their_own_shared_schedule() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let ids = [unique_id(), unique_id()];
    let mut config = scan_config();
    config.lower_to = Some(config.rank_to + 100);
    let circles: Vec<_> = ids
        .iter()
        .zip([100, config.rank_to + 5])
        .map(|(id, rank)| {
            json!({
                "circle_id": id,
                "name": "Lower Club",
                "monthly_rank": rank,
                "member_count": 9,
                "monthly_point": 20_000_000,
                "last_updated": "2026-03-19T15:00:00Z",
            })
        })
        .collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": circles })))
        .mount(&server)
        .await;
    mount_details(&server, &ids).await;
    let first = refreshing(&server, config, first_shared);
    let second = refreshing(&server, config, redis().await.unwrap());
    scan(&client(&server), &first, &config).await.unwrap();
    first.get(ids[1], Priority::Interactive).await.unwrap();

    let status = second.refresh_status().await;
    assert_eq!(status.scheduled, 1);
    assert_eq!((status.lower.scheduled, status.lower.loaded), (1, 1));
}

// One test, as every test shares the same key and would count each other's searches.
#[tokio::test]
async fn the_search_total_counts_successful_searches_and_never_expires() {
    let Some(shared) = redis().await else { return };
    let handle = install_metrics().unwrap();
    let other = redis().await.unwrap();
    let server = MockServer::start().await;
    mount_scan(&server, unique_id()).await;
    let running = instance(&server, scan_config(), shared.clone());
    running.wait_until(|s| s.snapshot.is_some()).await;
    let state = AppState {
        cache: running.cache.clone(),
        details: running.details.clone(),
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client(&server), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    };
    let search = |query: &'static str| {
        let app = router(state.clone());
        async move {
            app.oneshot(
                Request::get(format!("/api/v1/recommendations{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
        }
    };

    let before = shared.searches().await.unwrap();
    assert_eq!(search("?pace=-1").await, StatusCode::BAD_REQUEST);
    assert_eq!(search("?pace=5000000").await, StatusCode::OK);
    wait_for(async || shared.searches().await.unwrap() > before).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(shared.searches().await.unwrap(), before + 1);

    other.count_search().await;
    assert_eq!(shared.searches().await.unwrap(), before + 2);
    assert_eq!(ttl_millis("usm:v1:searches").await, -1);

    let response = metrics_router(handle, state)
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rendered = String::from_utf8(body.to_vec()).unwrap();
    let exported: u64 = rendered
        .lines()
        .find_map(|line| line.strip_prefix("searches_total "))
        .expect("searches_total is exported")
        .parse()
        .unwrap();
    assert_eq!(exported, before + 2);
    running.stop().await;
}

#[tokio::test]
async fn the_search_total_is_absent_without_redis() {
    let none = SharedCache::none();
    none.count_search().await;
    assert_eq!(none.searches().await, None);
}

/// One club's details: members `viewers`, each gaining `per_day` for `days` days of `month`.
fn rolling(id: u64, viewers: &[u64], month: u32, per_day: i64, days: usize) -> ResponseTemplate {
    let members: Vec<serde_json::Value> = viewers
        .iter()
        .map(|viewer| {
            let fans: Vec<i64> = (0..days)
                .map(|day| 100_000_000 + per_day * day as i64)
                .collect();
            json!({ "viewer_id": viewer, "year": 2026, "month": month, "daily_fans": fans })
        })
        .collect();
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": { "circle_id": id, "name": "Rolling Club", "member_count": viewers.len() },
        "members": members,
    }))
}

#[tokio::test]
async fn priors_reach_a_restarted_instance() {
    let Some(first_shared) = redis().await else {
        return;
    };
    let server = MockServer::start().await;
    let (old_club, new_club) = (unique_id(), unique_id());
    let viewers: Vec<u64> = (0..10).map(|_| unique_id()).collect();
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", old_club.to_string()))
        .respond_with(rolling(old_club, &viewers, 2, 2_000_000, 28))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", old_club.to_string()))
        .respond_with(rolling(old_club, &viewers, 3, 4_000_000, 2))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", new_club.to_string()))
        .respond_with(rolling(new_club, &viewers[..1], 3, 4_000_000, 2))
        .mount(&server)
        .await;

    let config = DetailConfig::default();
    let (first, _a) =
        ClubDetails::with_shared(client(&server), pool(old_club), config, first_shared);
    first.get(old_club, Priority::Interactive).await.unwrap();
    first
        .refresh(old_club, Priority::Interactive)
        .await
        .unwrap();

    // A new instance, with nothing in memory, finds the member's pace from February in Redis.
    let (second, _b) = ClubDetails::with_shared(
        client(&server),
        pool(new_club),
        config,
        redis().await.unwrap(),
    );
    let club = second.get(new_club, Priority::Interactive).await.unwrap();
    assert_eq!(club.members[0].prior, Some(2_000_000.0));
}
