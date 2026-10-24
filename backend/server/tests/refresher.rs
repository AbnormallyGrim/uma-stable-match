//! Refresher tests against a mock server. All data is made up.

use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use uma_stable_match_server::{
    scan, ClubDetails, DetailConfig, RefreshConfig, RefreshStatus, ScanConfig, SharedPool,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, Priority, RetryConfig};
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

fn scan_config(max_details: u32) -> ScanConfig {
    ScanConfig {
        rank_from: 1,
        rank_to: 50,
        max_details,
        page_size: 10,
    }
}

fn refreshing(server: &MockServer, refresh: RefreshConfig) -> ClubDetails {
    let config = DetailConfig {
        refresh,
        refresher: true,
        ..DetailConfig::default()
    };
    ClubDetails::new(&client(server), SharedPool::default(), config).0
}

fn every(millis: u64) -> RefreshConfig {
    RefreshConfig {
        every: Some(Duration::from_millis(millis)),
        ..RefreshConfig::default()
    }
}

fn listed(id: u64, rank: i32) -> Value {
    json!({
        "circle_id": id,
        "name": format!("Refresh Club {id}"),
        "monthly_rank": rank,
        "member_count": 20,
        "join_style": 1,
        "monthly_point": id as i64 * 1_000_000,
        "last_updated": "2026-03-19T15:00:00Z",
    })
}

async fn mount_list(server: &MockServer, circles: Vec<Value>) {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circles": circles,
            "total": 0,
            "page": 0,
            "limit": 10,
            "total_pages": 0,
        })))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

fn detail(id: u64, name: &str, last_updated: Option<DateTime<Utc>>) -> ResponseTemplate {
    let members: Vec<Value> = (0..9)
        .map(|i| {
            let fans: Vec<i64> = (0..4).map(|day| 100_000_000 + 1000 * day).collect();
            json!({ "viewer_id": 7001 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": {
            "circle_id": id,
            "name": name,
            "member_count": 9,
            "last_updated": last_updated.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true)),
        },
        "members": members,
    }))
}

async fn mount_detail(server: &MockServer, id: u64, response: ResponseTemplate) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn detail_calls(server: &MockServer) -> Vec<u64> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.url.path() == "/api/v4/circles")
        .filter_map(|r| {
            r.url
                .query_pairs()
                .find(|(k, _)| k == "circle_id")
                .and_then(|(_, v)| v.parse().ok())
        })
        .collect()
}

async fn wait_until(what: &str, mut done: impl AsyncFnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done().await {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn now() -> DateTime<Utc> {
    SystemTime::now().into()
}

fn start(details: &ClubDetails) {
    tokio::spawn(details.refresher().unwrap().run());
}

#[tokio::test]
async fn without_refresh_settings_there_is_no_refresher() {
    let server = MockServer::start().await;
    let (details, _) = ClubDetails::new(
        &client(&server),
        SharedPool::default(),
        DetailConfig::default(),
    );
    assert!(!details.refreshes());
    assert!(details.refresher().is_none());
}

#[tokio::test]
async fn a_scan_only_lists_and_the_refresher_loads_the_best_ranks_first() {
    let server = MockServer::start().await;
    mount_list(
        &server,
        vec![listed(3, 12), listed(1, 10), listed(4, 13), listed(2, 11)],
    )
    .await;
    for id in 1..=4 {
        mount_detail(&server, id, detail(id, "Listed", Some(now()))).await;
    }
    let details = refreshing(&server, RefreshConfig::default());

    let scan = scan(&client(&server), &details, &scan_config(3))
        .await
        .unwrap();
    assert_eq!(scan.candidates, 4);
    assert_eq!(scan.prefetched, 0);
    assert!(detail_calls(&server).await.is_empty());
    assert_eq!(details.scheduled().await, 3);
    assert_eq!(details.next_refresh(4).await, None);

    start(&details);
    wait_until("three clubs load", async || {
        details.cached_in_pool().len() == 3
    })
    .await;
    assert_eq!(detail_calls(&server).await, vec![1, 2, 3]);
}

#[tokio::test]
async fn schedules_the_next_refresh_after_uma_moes_daily_update() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10)]).await;
    let updated = now() - Duration::from_secs(60 * 60);
    let updated = DateTime::from_timestamp(updated.timestamp(), 0).unwrap();
    mount_detail(&server, 1, detail(1, "Fresh", Some(updated))).await;
    let details = refreshing(&server, RefreshConfig::default());
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    start(&details);
    let due = updated + Duration::from_secs(24 * 60 * 60 + 30 * 60);
    wait_until("the next refresh is set", async || {
        details.next_refresh(1).await == Some(due)
    })
    .await;
    assert_eq!(details.last_updated(1).await, Some(updated));
}

#[tokio::test]
async fn rechecks_a_club_uma_moe_has_not_updated_yet() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10)]).await;
    let stale = now() - Duration::from_secs(25 * 60 * 60);
    mount_detail(&server, 1, detail(1, "Stale", Some(stale))).await;
    let details = refreshing(&server, RefreshConfig::default());
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    let before = now();
    start(&details);
    wait_until("the club loads", async || details.cached(1).await.is_some()).await;
    wait_until("the recheck is set", async || {
        details
            .next_refresh(1)
            .await
            .is_some_and(|due| due >= before + Duration::from_secs(60 * 60 - 1))
    })
    .await;
    assert!(details.next_refresh(1).await.unwrap() <= now() + Duration::from_secs(60 * 60));
}

#[tokio::test]
async fn refreshes_replace_the_details_on_the_interval() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10)]).await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(detail(1, "Before", Some(now())))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_detail(&server, 1, detail(1, "After", Some(now()))).await;
    let details = refreshing(&server, every(100));
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    start(&details);
    wait_until("the first load", async || details.cached(1).await.is_some()).await;
    assert_eq!(details.cached(1).await.unwrap().name, "Before");
    wait_until("the refresh", async || {
        details.cached(1).await.is_some_and(|c| c.name == "After")
    })
    .await;
}

#[tokio::test]
async fn retries_a_failed_refresh_and_keeps_serving_the_old_details() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10)]).await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(detail(1, "Before", Some(now())))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_detail(&server, 1, detail(1, "After", Some(now()))).await;
    let details = refreshing(
        &server,
        RefreshConfig {
            every: Some(Duration::from_millis(50)),
            retry: Duration::from_millis(300),
            ..RefreshConfig::default()
        },
    );
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    start(&details);
    wait_until("the refresh fails", async || {
        detail_calls(&server).await.len() == 2
    })
    .await;
    assert_eq!(details.cached(1).await.unwrap().name, "Before");
    let retry = details.next_refresh(1).await.unwrap();
    assert!(retry > now() + Duration::from_millis(100));
    wait_until("the retry", async || {
        details.cached(1).await.is_some_and(|c| c.name == "After")
    })
    .await;
}

#[tokio::test]
async fn current_details_are_not_fetched_again() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10), listed(2, 11)]).await;
    mount_detail(&server, 1, detail(1, "Loaded", Some(now()))).await;
    mount_detail(&server, 2, detail(2, "Listed", Some(now()))).await;
    let details = refreshing(&server, RefreshConfig::default());
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();
    details.get(1, Priority::Interactive).await.unwrap();

    start(&details);
    wait_until("both clubs load", async || {
        details.cached_in_pool().len() == 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(detail_calls(&server).await, vec![1, 2]);
    assert!(details.next_refresh(1).await.unwrap() > now() + Duration::from_secs(60 * 60));
}

#[tokio::test]
async fn clubs_loaded_for_a_search_join_the_schedule() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10), listed(2, 11)]).await;
    let updated = DateTime::from_timestamp(now().timestamp(), 0).unwrap();
    mount_detail(&server, 2, detail(2, "Searched", Some(updated))).await;
    let details = refreshing(&server, RefreshConfig::default());
    scan(&client(&server), &details, &scan_config(1))
        .await
        .unwrap();
    assert_eq!(details.next_refresh(2).await, None);

    details.get(2, Priority::Interactive).await.unwrap();
    assert_eq!(
        details.next_refresh(2).await,
        Some(updated + Duration::from_secs(24 * 60 * 60 + 30 * 60))
    );
    assert_eq!(details.scheduled().await, 2);
}

#[tokio::test]
async fn a_new_listing_drops_clubs_that_left_and_adds_new_ones() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10), listed(2, 11)]).await;
    let details = refreshing(&server, RefreshConfig::default());
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();
    assert!(details.next_refresh(1).await.is_some());
    assert!(details.next_refresh(2).await.is_some());

    mount_list(&server, vec![listed(2, 10), listed(3, 11)]).await;
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();
    assert_eq!(details.next_refresh(1).await, None);
    assert!(details.next_refresh(2).await.is_some());
    assert!(details.next_refresh(3).await.is_some());
    assert_eq!(details.scheduled().await, 2);
}

#[tokio::test]
async fn refreshed_details_do_not_expire() {
    let server = MockServer::start().await;
    mount_detail(&server, 1, detail(1, "Kept", Some(now()))).await;
    let config = DetailConfig {
        refresh: RefreshConfig {
            every: Some(Duration::from_millis(50)),
            ..RefreshConfig::default()
        },
        refresher: true,
        ..DetailConfig::default()
    };
    let (details, _) = ClubDetails::new(&client(&server), SharedPool::default(), config);

    details.get(1, Priority::Interactive).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(details.cached(1).await.is_some());
}

#[tokio::test]
async fn ranks_past_the_fast_ranks_refresh_at_half_pace() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10), listed(2, 11)]).await;
    for id in [1, 2] {
        let response = detail(id, "Slow", Some(now())).set_delay(Duration::from_millis(150));
        mount_detail(&server, id, response).await;
    }
    let details = refreshing(
        &server,
        RefreshConfig {
            fast_ranks: 0,
            ..RefreshConfig::default()
        },
    );
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    let started = Instant::now();
    start(&details);
    wait_until("both clubs load", async || {
        details.cached_in_pool().len() == 2
    })
    .await;
    assert!(started.elapsed() >= Duration::from_millis(450));
}

#[tokio::test]
async fn reports_scheduled_loaded_and_due_clubs() {
    let server = MockServer::start().await;
    mount_list(&server, vec![listed(1, 10), listed(2, 11), listed(3, 12)]).await;
    for id in 1..=3 {
        mount_detail(&server, id, detail(id, "Counted", Some(now()))).await;
    }
    let details = refreshing(&server, RefreshConfig::default());
    assert_eq!(details.refresh_status().await, RefreshStatus::default());
    let before = now();
    scan(&client(&server), &details, &scan_config(10))
        .await
        .unwrap();

    let status = details.refresh_status().await;
    assert_eq!((status.scheduled, status.loaded, status.due), (3, 0, 3));
    assert!(status.oldest_due.is_some_and(|due| due >= before));
    assert_eq!(status.stored_ttl, None);
    details.get(2, Priority::Interactive).await.unwrap();
    assert_eq!(details.refresh_status().await.loaded, 1);

    start(&details);
    wait_until("every club loads", async || {
        details.refresh_status().await.loaded == 3
    })
    .await;
    let status = details.refresh_status().await;
    assert_eq!(
        (status.scheduled, status.due, status.oldest_due),
        (3, 0, None)
    );
}
