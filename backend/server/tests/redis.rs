//! Redis shared-cache tests. All data is made up.
//!
//! The tests marked as needing Redis run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it. The others use a fake server.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uma_stable_match_server::{
    club_key, run_scheduler, Cache, CacheStatus, ClubDetails, ClubPool, DetailConfig, KeySecret,
    ListedClub, PaceLookup, RedisUrl, ScanConfig, SharedCache, SharedPool, DEFAULT_PACE_TTL,
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
        ClubDetails::with_shared(&client(&server), pool(id), DetailConfig::default(), shared);

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

    let config = DetailConfig {
        ttl: Duration::from_secs(60),
        ..DetailConfig::default()
    };
    let (first, _a) = ClubDetails::with_shared(&client(&server), pool(id), config, first_shared);
    let (second, _b) =
        ClubDetails::with_shared(&client(&server), pool(id), config, second_shared.clone());

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
    assert_eq!(first.pace(trainer, None).await.unwrap(), 3_000_000.0);
    assert_eq!(second.pace(trainer, None).await.unwrap(), 3_000_000.0);

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
        &client(server),
        SharedPool::default(),
        DetailConfig {
            ttl: Duration::from_secs(60),
            ..DetailConfig::default()
        },
        shared,
    );
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server),
        details.clone(),
        config,
        Duration::from_secs(3600),
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
