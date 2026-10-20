//! Club detail cache tests against a mock server. All data is made up.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::{json, Value};
use uma_stable_match_server::{
    BackgroundLoader, ClubDetails, ClubPool, DetailConfig, ListedClub, SharedPool,
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
        ttl: Duration::from_millis(100),
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
