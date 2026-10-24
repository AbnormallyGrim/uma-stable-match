//! Club detail cache tests against a mock server. All data is made up.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::{json, Value};
use uma_stable_match_server::{
    BackgroundLoader, ClubDetails, ClubPool, DetailConfig, ListedClub, RefreshConfig, SharedPool,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, Priority, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer, min_interval: Duration) -> Client {
    Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn pool(ids: &[u64]) -> SharedPool {
    Arc::new(RwLock::new(ClubPool::new(ids.iter().map(|id| {
        ListedClub {
            circle_id: *id,
            member_count: 9,
            join_style: None,
            raw_rate: 1000.0,
            policy: None,
        }
    }))))
}

fn details(
    server: &MockServer,
    config: DetailConfig,
    ids: &[u64],
) -> (ClubDetails, BackgroundLoader) {
    ClubDetails::new(&client(server, Duration::ZERO), pool(ids), config)
}

fn detail(id: u64) -> Value {
    let members: Vec<Value> = [100, 200, 300, 400, 500, 600, 700, 800, 5000]
        .iter()
        .enumerate()
        .map(|(i, gain)| {
            let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + gain * day).collect();
            json!({ "viewer_id": 7001 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    json!({
        "circle": { "circle_id": id, "name": format!("Detail Club {id}"), "member_count": 9 },
        "members": members,
    })
}

async fn mount(server: &MockServer, id: u64, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn ok(id: u64) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(detail(id))
}

#[tokio::test]
async fn concurrent_requests_share_one_fetch() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1).set_delay(Duration::from_millis(200)), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    let gets = (0..5).map(|_| {
        let details = details.clone();
        tokio::spawn(async move { details.get(1, Priority::Interactive).await })
    });
    for get in gets {
        assert_eq!(get.await.unwrap().unwrap().circle_id, 1);
    }
}

#[tokio::test]
async fn cached_clubs_are_not_fetched_again() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    assert!(details.cached(1).await.is_none());
    details.get(1, Priority::Interactive).await.unwrap();
    details.get(1, Priority::Background).await.unwrap();
    assert_eq!(details.cached(1).await.unwrap().name, "Detail Club 1");
}

#[tokio::test]
async fn expired_clubs_are_fetched_again() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1), 2).await;
    let config = DetailConfig {
        refresh: RefreshConfig {
            every: Some(Duration::from_millis(100)),
            ..RefreshConfig::default()
        },
        ..DetailConfig::default()
    };
    let (details, _loader) = details(&server, config, &[1]);

    details.get(1, Priority::Interactive).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    details.get(1, Priority::Interactive).await.unwrap();
}

#[tokio::test]
async fn failed_loads_are_not_cached() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    mount(&server, 1, ok(1), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    assert!(details.get(1, Priority::Interactive).await.is_err());
    assert!(details.cached(1).await.is_none());
    assert!(details.get(1, Priority::Interactive).await.is_ok());
}

#[tokio::test]
async fn missing_club_ids_are_filled_in() {
    let server = MockServer::start().await;
    let mut body = detail(1);
    body["circle"] = json!({});
    mount(
        &server,
        1,
        ResponseTemplate::new(200).set_body_json(body),
        1,
    )
    .await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    assert_eq!(
        details
            .get(1, Priority::Interactive)
            .await
            .unwrap()
            .circle_id,
        1
    );
}

#[tokio::test]
async fn loads_set_exact_medians_in_the_pool() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1, 2]);

    details.get(1, Priority::Interactive).await.unwrap();
    let pool = details.pool().read().unwrap();
    assert_eq!(pool.estimated_median(1), Some(500.0));
    assert_eq!(pool.estimated_median(2), Some(1000.0));
    assert_eq!(pool.exact_medians(), 1);
}

#[tokio::test]
async fn a_new_pool_takes_cached_medians() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);
    details.get(1, Priority::Interactive).await.unwrap();

    let mut next = pool(&[1, 3]).read().unwrap().clone();
    assert_eq!(details.restore_exact_medians(&mut next), 1);
    assert_eq!(next.estimated_median(1), Some(500.0));
    assert_eq!(next.estimated_median(3), Some(1000.0));
}

#[tokio::test]
async fn background_loader_fetches_queued_clubs_once() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1), 1).await;
    mount(&server, 2, ok(2), 1).await;
    let (details, loader) = details(&server, DetailConfig::default(), &[1, 2]);
    let task = tokio::spawn(loader.run());

    assert!(details.queue(1));
    assert!(!details.queue(1));
    assert!(details.queue(2));
    for _ in 0..100 {
        if details.queued() == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(details.queued(), 0);
    assert!(details.cached(2).await.is_some());
    assert!(!details.queue(1));
    task.abort();
}

#[tokio::test]
async fn background_failures_leave_the_queue() {
    let server = MockServer::start().await;
    mount(&server, 1, ResponseTemplate::new(500), 1).await;
    let (details, loader) = details(&server, DetailConfig::default(), &[1]);
    let task = tokio::spawn(loader.run());

    assert!(details.queue(1));
    for _ in 0..100 {
        if details.queued() == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(details.queued(), 0);
    assert!(details.cached(1).await.is_none());
    task.abort();
}

#[tokio::test]
async fn visitors_go_before_background_loads() {
    let server = MockServer::start().await;
    for id in 1..=4 {
        mount(&server, id, ok(id), 1).await;
    }
    let client = client(&server, Duration::from_millis(200));
    let (details, loader) = ClubDetails::new(&client, pool(&[1, 2, 3, 4]), DetailConfig::default());
    let task = tokio::spawn(loader.run());

    for id in 1..=3 {
        assert!(details.queue(id));
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    details.get(4, Priority::Interactive).await.unwrap();
    for _ in 0..200 {
        if details.queued() == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    task.abort();

    let order: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.url.query().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        order,
        vec!["circle_id=1", "circle_id=4", "circle_id=2", "circle_id=3"]
    );
}

#[tokio::test]
async fn cached_clubs_outside_the_pool_are_left_out() {
    let server = MockServer::start().await;
    for id in 1..=3 {
        mount(&server, id, ok(id), 1).await;
    }
    let (details, _loader) = details(&server, DetailConfig::default(), &[1, 2, 3]);
    for id in [3, 1, 2] {
        details.get(id, Priority::Interactive).await.unwrap();
    }

    let ids = |details: &ClubDetails| -> Vec<u64> {
        details
            .cached_in_pool()
            .iter()
            .map(|c| c.circle_id)
            .collect()
    };
    assert_eq!(ids(&details), vec![1, 2, 3]);
    details.replace_pool(pool(&[2, 3]).read().unwrap().clone());
    assert_eq!(ids(&details), vec![2, 3]);
    assert!(details.cached(1).await.is_some());
}

#[tokio::test]
async fn debug_shows_counts_only() {
    let server = MockServer::start().await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[]);
    assert_eq!(
        format!("{details:?}"),
        "ClubDetails { cached: 0, queued: 0, .. }"
    );
}

#[tokio::test]
async fn months_are_cached_separately() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .and(query_param("month", "2"))
        .respond_with(ok(1))
        .expect(1)
        .mount(&server)
        .await;
    mount(&server, 1, ok(1), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    details.get(1, Priority::Interactive).await.unwrap();
    details.set_period(Some((2026, 2)));
    assert!(details.cached(1).await.is_none());
    assert!(details.cached_in_pool().is_empty());
    assert!(details.queue(1));

    details.get(1, Priority::Interactive).await.unwrap();
    assert_eq!(details.cached_in_pool().len(), 1);
    details.set_period(None);
    assert!(details.cached(1).await.is_some());
}

#[tokio::test]
async fn loads_from_an_old_month_leave_the_pool_alone() {
    let server = MockServer::start().await;
    mount(&server, 1, ok(1).set_delay(Duration::from_millis(200)), 1).await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[1]);

    let loading = {
        let details = details.clone();
        tokio::spawn(async move { details.get(1, Priority::Interactive).await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    details.set_period(Some((2026, 2)));
    loading.await.unwrap().unwrap();

    assert_eq!(details.pool().read().unwrap().exact_medians(), 0);
}

fn detail_with(id: u64, name: &str, gain: i64, last_updated: Option<&str>) -> ResponseTemplate {
    let members: Vec<Value> = (0..9)
        .map(|i| {
            let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + gain * day).collect();
            json!({ "viewer_id": 7001 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": {
            "circle_id": id,
            "name": name,
            "member_count": 9,
            "last_updated": last_updated,
        },
        "members": members,
    }))
}

#[tokio::test]
async fn refresh_replaces_a_cached_club() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000401"))
        .respond_with(detail_with(900000401, "Before", 1_000, None))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000401"))
        .respond_with(detail_with(900000401, "After", 2_000, None))
        .mount(&server)
        .await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[900000401]);

    assert_eq!(
        details
            .get(900000401, Priority::Interactive)
            .await
            .unwrap()
            .name,
        "Before"
    );
    let refreshed = details
        .refresh(900000401, Priority::Background)
        .await
        .unwrap();
    assert_eq!(refreshed.name, "After");
    assert_eq!(
        details
            .get(900000401, Priority::Interactive)
            .await
            .unwrap()
            .name,
        "After"
    );
    assert_eq!(
        details.pool().read().unwrap().estimated_median(900000401),
        Some(2_000.0)
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_failed_refresh_keeps_the_cached_club() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000402"))
        .respond_with(detail_with(900000402, "Kept", 1_000, None))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900000402"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[900000402]);

    details.get(900000402, Priority::Interactive).await.unwrap();
    details
        .refresh(900000402, Priority::Background)
        .await
        .unwrap_err();
    assert_eq!(details.cached(900000402).await.unwrap().name, "Kept");
}

#[tokio::test]
async fn remembers_when_uma_moe_last_updated_a_club() {
    let server = MockServer::start().await;
    mount(
        &server,
        900000403,
        detail_with(900000403, "Dated", 1_000, Some("2026-03-19T15:00:00Z")),
        1,
    )
    .await;
    mount(
        &server,
        900000404,
        detail_with(900000404, "Undated", 1_000, None),
        1,
    )
    .await;
    let (details, _loader) = details(&server, DetailConfig::default(), &[900000403, 900000404]);

    assert_eq!(details.last_updated(900000403).await, None);
    details.get(900000403, Priority::Interactive).await.unwrap();
    details.get(900000404, Priority::Interactive).await.unwrap();
    assert_eq!(
        details.last_updated(900000403).await,
        Some("2026-03-19T15:00:00Z".parse().unwrap())
    );
    assert_eq!(details.last_updated(900000404).await, None);
}

fn recheck(millis: u64) -> DetailConfig {
    DetailConfig {
        refresh: RefreshConfig {
            recheck: Duration::from_millis(millis),
            ..RefreshConfig::default()
        },
        ..DetailConfig::default()
    }
}

#[tokio::test]
async fn details_uma_moe_has_not_updated_are_checked_again() {
    let server = MockServer::start().await;
    mount(
        &server,
        1,
        detail_with(1, "Stale", 1000, Some("2026-03-19T15:00:00Z")),
        2,
    )
    .await;
    let (details, _loader) = details(&server, recheck(100), &[1]);

    details.get(1, Priority::Interactive).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    details.get(1, Priority::Interactive).await.unwrap();
}

#[tokio::test]
async fn details_stay_cached_until_uma_moes_next_update() {
    let server = MockServer::start().await;
    let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    let last_updated = (now - Duration::from_secs(3600)).to_rfc3339();
    mount(
        &server,
        1,
        detail_with(1, "Fresh", 1000, Some(&last_updated)),
        1,
    )
    .await;
    let (details, _loader) = details(&server, recheck(100), &[1]);

    details.get(1, Priority::Interactive).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    details.get(1, Priority::Interactive).await.unwrap();
}
