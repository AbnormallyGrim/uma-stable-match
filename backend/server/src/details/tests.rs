use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, StatusCode};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn details(queue_size: usize) -> (ClubDetails, BackgroundLoader) {
    let config = ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    };
    let client = Client::new(config).unwrap();
    let detail_config = DetailConfig {
        queue_size,
        ..DetailConfig::default()
    };
    ClubDetails::new(&client, SharedPool::default(), detail_config)
}

#[tokio::test]
async fn queue_skips_duplicates() {
    let (details, _loader) = details(10);
    assert!(details.queue(1));
    assert!(!details.queue(1));
    assert!(details.queue(2));
    assert_eq!(details.queued(), 2);
}

#[tokio::test]
async fn queue_is_bounded() {
    let (details, _loader) = details(2);
    assert!(details.queue(1));
    assert!(details.queue(2));
    assert!(!details.queue(3));
    assert_eq!(details.queued(), 2);
}

#[test]
fn default_config() {
    let config = DetailConfig::default();
    assert_eq!(config.refresh, RefreshConfig::default());
    assert!(!config.refresher);
    assert!(config.max_bytes > 0);
    assert!(config.queue_size > 0);
}

#[tokio::test]
async fn visitor_fetches_give_up_on_a_long_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    let (details, _loader) =
        ClubDetails::new(&client, SharedPool::default(), DetailConfig::default());

    let started = Instant::now();
    let error = details
        .get(900_000_101, Priority::Interactive)
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::TOO_MANY_REQUESTS));
    assert!(started.elapsed() < Duration::from_secs(5));
}

fn stored_club(id: u64, gain: i64) -> Club {
    let members: Vec<_> = (1..=9)
        .map(|i| {
            let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + gain * i * day).collect();
            serde_json::json!({ "viewer_id": 7000 + i, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    let response: uma_stable_match_umamoe_client::CircleResponse = serde_json::from_value(
        serde_json::json!({ "circle": { "circle_id": id, "name": "Stored Club", "member_count": 9 }, "members": members }),
    )
    .unwrap();
    Club::from(response)
}

// Needs Redis: runs only when `REDIS_TEST_URL` is set, as in `tests/redis.rs`.
#[tokio::test]
async fn clubs_read_from_redis_set_their_exact_medians() {
    let Some(url) = std::env::var("REDIS_TEST_URL")
        .ok()
        .and_then(|url| crate::shared::RedisUrl::parse(&url))
    else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return;
    };
    let secret = crate::shared::KeySecret::parse("made-up-secret-made-up-secret-0002").unwrap();
    let shared = SharedCache::connect(&url, secret).await.unwrap();
    let ids: Vec<u64> = (0..3)
        .map(|_| fastrand::u64(100_000_000_000..900_000_000_000))
        .collect();
    let due = (utc_now() + chrono::TimeDelta::hours(1)).timestamp_millis();
    let mut expected = Vec::new();
    for (i, &id) in ids.iter().enumerate() {
        let club = stored_club(id, 100 * (i as i64 + 1));
        expected.push(crate::calibration::club_median(&club).unwrap());
        let stored = Stored {
            club,
            last_updated: None,
            due_ms: Some(due),
        };
        shared
            .set(&club_key(id, None), &stored, Duration::from_secs(60))
            .await;
    }
    let pool = SharedPool::new(RwLock::new(ClubPool::new(ids.iter().map(|&circle_id| {
        crate::pool::ListedClub {
            circle_id,
            member_count: 9,
            join_style: None,
            policy: None,
            discord: false,
            raw_rate: 1000.0,
        }
    }))));
    let client = Client::new(ClientConfig::new(ApiKey::new("test-key"))).unwrap();
    let (details, _loader) =
        ClubDetails::with_shared(&client, pool, DetailConfig::default(), shared);

    assert_eq!(details.load_shared(&ids).await.len(), 3);
    let pool = details.pool().read().unwrap();
    assert_eq!(pool.exact_medians(), 3);
    for (&id, median) in ids.iter().zip(expected) {
        assert_eq!(pool.estimated_median(id), Some(median));
    }
}

#[test]
fn club_weight_grows_with_member_data() {
    let empty = Club::new(1, "Empty Club");
    let full = stored_club(1, 100);
    assert!(club_weight(&full) > club_weight(&empty));
    let fans: usize = full.members.iter().map(|m| m.daily_fans.len()).sum();
    assert!(club_weight(&full) as usize >= fans * size_of::<i64>());
}

#[tokio::test]
async fn cache_stays_within_its_byte_limit() {
    let server = MockServer::start().await;
    for id in 1..=6u64 {
        let members: Vec<_> = (1..=9i64)
            .map(|i| {
                let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + 100 * i * day).collect();
                serde_json::json!({ "viewer_id": 7000 + i, "year": 2026, "month": 3, "daily_fans": fans })
            })
            .collect();
        Mock::given(path("/api/v4/circles"))
            .and(wiremock::matchers::query_param("circle_id", id.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "circle": { "circle_id": id, "name": "Weighed Club", "member_count": 9 },
                "members": members,
            })))
            .mount(&server)
            .await;
    }
    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    let max_bytes = 2 * u64::from(club_weight(&stored_club(1, 100))) + 1;
    let config = DetailConfig {
        max_bytes,
        ..DetailConfig::default()
    };
    let (details, _loader) = ClubDetails::new(&client, SharedPool::default(), config);

    for id in 1..=6 {
        details.get(id, Priority::Interactive).await.unwrap();
        details.settle().await;
    }
    assert!(details.cached_bytes() <= max_bytes);
    assert!(details.cached_clubs() < 6);
}
