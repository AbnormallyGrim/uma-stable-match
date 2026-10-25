//! Scanner tests against a mock server. All data is made up.

use std::time::Duration;

use serde_json::{json, Value};
use uma_stable_match_server::{
    scan, ClubDetails, DetailConfig, Scan, ScanConfig, ScanError, SharedPool,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
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

fn config() -> ScanConfig {
    ScanConfig {
        rank_from: 10,
        rank_to: 50,
        max_details: 10,
        page_size: 3,
        lower_to: None,
    }
}

fn listed(id: u64, rank: i32, members: i32) -> Value {
    json!({
        "circle_id": id,
        "name": format!("Scan Club {id}"),
        "monthly_rank": rank,
        "member_count": members,
        "join_style": 1,
        "monthly_point": id as i64 * 1_000_000,
        "last_updated": "2026-03-19T15:00:00Z",
    })
}

fn details(server: &MockServer) -> ClubDetails {
    ClubDetails::new(
        &client(server),
        SharedPool::default(),
        DetailConfig::default(),
    )
    .0
}

async fn run(server: &MockServer, config: &ScanConfig) -> (Result<Scan, ScanError>, ClubDetails) {
    let details = details(server);
    (scan(&client(server), &details, config).await, details)
}

fn page(circles: Vec<Value>) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circles": circles,
        "total": 0,
        "page": 0,
        "limit": 3,
        "total_pages": 0,
    }))
}

fn detail(id: u64) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": { "circle_id": id, "name": format!("Scan Club {id}"), "member_count": 20 },
        "members": [{ "viewer_id": 7001, "year": 2026, "month": 3, "daily_fans": [100, 200, 300] }],
        "club_rank": 5,
    }))
}

async fn mount_page(server: &MockServer, n: u32, circles: Vec<Value>) {
    Mock::given(method("GET"))
        .and(path("/api/v4/circles/list"))
        .and(query_param("page", n.to_string()))
        .and(query_param("limit", "3"))
        .and(query_param("max_rank", "50"))
        .and(query_param("sort_by", "monthly_rank"))
        .and(query_param("sort_dir", "asc"))
        .respond_with(page(circles))
        .expect(1)
        .mount(server)
        .await;
}

async fn mount_detail(server: &MockServer, id: u64, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", id.to_string()))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn ids(details: &ClubDetails) -> Vec<u64> {
    details
        .cached_in_pool()
        .iter()
        .map(|c| c.circle_id)
        .collect()
}

#[tokio::test]
async fn pages_until_past_the_window() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 5, 20), listed(2, 12, 20), listed(3, 20, 20)],
    )
    .await;
    mount_page(
        &server,
        1,
        vec![listed(4, 30, 20), listed(5, 40, 20), listed(6, 50, 20)],
    )
    .await;
    for id in 2..=6 {
        mount_detail(&server, id, detail(id), 1).await;
    }

    let (scan, details) = run(&server, &config()).await;
    let scan = scan.unwrap();
    assert_eq!(scan.candidates, 5);
    assert_eq!(scan.failed, 0);
    assert_eq!(ids(&details), vec![2, 3, 4, 5, 6]);
}

#[tokio::test]
async fn stops_on_a_short_page() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20), listed(2, 13, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let (scan, details) = run(&server, &config()).await;
    scan.unwrap();
    assert_eq!(ids(&details), vec![1, 2]);
}

#[tokio::test]
async fn stops_on_an_empty_page() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 20), listed(2, 13, 20), listed(3, 14, 20)],
    )
    .await;
    mount_page(&server, 1, vec![]).await;
    for id in 1..=3 {
        mount_detail(&server, id, detail(id), 1).await;
    }

    let (scan, _) = run(&server, &config()).await;
    let scan = scan.unwrap();
    assert_eq!(scan.candidates, 3);
}

#[tokio::test]
async fn skips_full_clubs_and_unknown_sizes() {
    let server = MockServer::start().await;
    let mut unknown = listed(3, 14, 20);
    unknown["member_count"] = Value::Null;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 30), listed(2, 13, 29), unknown],
    )
    .await;
    mount_page(&server, 1, vec![]).await;
    mount_detail(&server, 1, detail(1), 0).await;
    mount_detail(&server, 2, detail(2), 1).await;
    mount_detail(&server, 3, detail(3), 0).await;

    let (scan, details) = run(&server, &config()).await;
    scan.unwrap();
    assert_eq!(ids(&details), vec![2]);
}

#[tokio::test]
async fn prefetches_the_best_ranks_first() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 20), listed(2, 13, 20), listed(3, 14, 20)],
    )
    .await;
    mount_page(&server, 1, vec![listed(4, 15, 20), listed(5, 16, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;
    for id in [3, 4, 5] {
        mount_detail(&server, id, detail(id), 0).await;
    }

    let mut config = config();
    config.max_details = 2;
    let (scan, details) = run(&server, &config).await;
    let scan = scan.unwrap();
    assert_eq!(scan.candidates, 5);
    assert_eq!(scan.prefetched, 2);
    assert_eq!(ids(&details), vec![1, 2]);
}

#[tokio::test]
async fn filters_before_the_cap() {
    let server = MockServer::start().await;
    let mut archived = listed(1, 12, 20);
    archived["archived"] = json!(true);
    mount_page(
        &server,
        0,
        vec![archived, listed(2, 13, 30), listed(3, 14, 7)],
    )
    .await;
    mount_page(
        &server,
        1,
        vec![listed(4, 15, 20), listed(5, 16, 20), listed(6, 17, 20)],
    )
    .await;
    mount_page(&server, 2, vec![]).await;
    for id in [1, 2, 3, 6] {
        mount_detail(&server, id, detail(id), 0).await;
    }
    mount_detail(&server, 4, detail(4), 1).await;
    mount_detail(&server, 5, detail(5), 1).await;

    let mut config = config();
    config.max_details = 2;
    let (scan, details) = run(&server, &config).await;
    let scan = scan.unwrap();
    assert_eq!(scan.candidates, 4);
    assert_eq!(ids(&details), vec![4, 5]);
}

#[tokio::test]
async fn keeps_closed_clubs() {
    let server = MockServer::start().await;
    let mut closed = listed(1, 12, 20);
    closed["join_style"] = json!(3);
    mount_page(&server, 0, vec![closed, listed(2, 13, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let (scan, details) = run(&server, &config()).await;
    scan.unwrap();
    assert_eq!(ids(&details), vec![1, 2]);
}

#[tokio::test]
async fn skips_clubs_that_fail_to_load() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20), listed(2, 13, 20)]).await;
    mount_detail(&server, 1, ResponseTemplate::new(500), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let (scan, details) = run(&server, &config()).await;
    let scan = scan.unwrap();
    assert_eq!(scan.failed, 1);
    assert_eq!(ids(&details), vec![2]);
}

#[tokio::test]
async fn converts_details_to_clubs() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;

    let (scan, details) = run(&server, &config()).await;
    assert_eq!(scan.unwrap().prefetched, 1);
    let club = details.cached(1).await.unwrap();
    assert_eq!(club.club_rank, Some(5));
    assert_eq!(club.members.len(), 1);
    assert_eq!(club.members[0].daily_fans, vec![100, 200, 300]);
}

#[tokio::test]
async fn falls_back_to_listed_details_when_missing() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20)]).await;
    mount_detail(
        &server,
        1,
        ResponseTemplate::new(200).set_body_json(json!({ "members": [], "club_rank": 4 })),
        1,
    )
    .await;

    let (scan, details) = run(&server, &config()).await;
    scan.unwrap();
    let club = details.cached(1).await.unwrap();
    assert_eq!(club.circle_id, 1);
    assert_eq!(club.name, "Scan Club 1");
    assert_eq!(club.monthly_rank, Some(12));
    assert_eq!(club.club_rank, Some(4));
}

#[tokio::test]
async fn list_failure_fails_the_scan() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let error = run(&server, &config()).await.0.unwrap_err();
    assert!(matches!(error, ScanError::List(_)));
}

#[tokio::test]
async fn skips_rows_without_points() {
    let server = MockServer::start().await;
    let mut no_points = listed(1, 12, 20);
    no_points["monthly_point"] = Value::Null;
    mount_page(&server, 0, vec![no_points, listed(2, 13, 20)]).await;
    mount_detail(&server, 1, detail(1), 0).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let (scan, details) = run(&server, &config()).await;
    assert_eq!(scan.unwrap().candidates, 1);
    assert_eq!(ids(&details), vec![2]);
}

#[tokio::test]
async fn reports_the_game_month() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;

    let (scan, _) = run(&server, &config()).await;
    let scan = scan.unwrap();
    assert_eq!(scan.month.as_deref(), Some("2026-03"));
    assert_eq!(
        scan.updated_at.map(|t| t.to_rfc3339()).as_deref(),
        Some("2026-03-19T15:00:00+00:00")
    );
}

#[tokio::test]
async fn a_new_scan_replaces_the_pool_and_reuses_cached_details() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(page(vec![listed(1, 12, 20), listed(2, 13, 20)]))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(page(vec![
            listed(1, 12, 30),
            listed(2, 13, 20),
            listed(3, 14, 20),
            listed(9, 60, 20),
        ]))
        .expect(1)
        .mount(&server)
        .await;
    for id in 1..=3 {
        mount_detail(&server, id, detail(id), 1).await;
    }

    let details = details(&server);
    scan(&client(&server), &details, &config()).await.unwrap();
    assert_eq!(ids(&details), vec![1, 2]);
    let second = scan(&client(&server), &details, &config()).await.unwrap();
    assert_eq!(second.candidates, 3);
    assert_eq!(second.prefetched, 2);
    assert_eq!(ids(&details), vec![1, 2, 3]);
    assert!(!details.pool().read().unwrap().is_joinable(1));
    assert!(details.cached(1).await.is_some());
}

#[tokio::test]
async fn a_new_scan_keeps_the_previous_pools_medians() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20), listed(2, 13, 20)]).await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let details = details(&server);
    scan(&client(&server), &details, &config()).await.unwrap();
    // Recorded earlier, but no longer cached.
    details.pool().write().unwrap().set_exact_median(1, 123.0);
    server.reset().await;
    mount_page(&server, 0, vec![listed(1, 12, 20), listed(2, 13, 20)]).await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    scan(&client(&server), &details, &config()).await.unwrap();

    let pool = details.pool().read().unwrap();
    assert_eq!(pool.estimated_median(1), Some(123.0));
    assert_eq!(pool.exact_medians(), 1);
}

#[tokio::test]
async fn a_new_month_drops_the_previous_pools_medians() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20)]).await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let details = details(&server);
    scan(&client(&server), &details, &config()).await.unwrap();
    details.pool().write().unwrap().set_exact_median(1, 123.0);
    server.reset().await;
    mount_page(&server, 0, vec![early(1, 12)]).await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let second = scan(&client(&server), &details, &config()).await.unwrap();

    assert_eq!(second.month.as_deref(), Some("2026-02"));
    assert_eq!(details.pool().read().unwrap().exact_medians(), 0);
}

fn early(id: u64, rank: i32) -> Value {
    let mut row = listed(id, rank, 20);
    row["last_updated"] = json!("2026-03-02T15:00:00Z");
    row["monthly_point"] = json!(id as i64 * 10_000);
    row["last_month_point"] = json!(id as i64 * 56_000_000);
    row
}

#[tokio::test]
async fn early_in_the_month_uses_last_month() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![early(1, 12), early(2, 13)]).await;
    for id in 1..=2 {
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .and(query_param("year", "2026"))
            .and(query_param("month", "2"))
            .respond_with(detail(id))
            .expect(1)
            .mount(&server)
            .await;
    }

    let (scan, details) = run(&server, &config()).await;
    let scan = scan.unwrap();
    assert_eq!(scan.month.as_deref(), Some("2026-02"));
    assert_eq!(scan.candidates, 2);
    assert_eq!(scan.prefetched, 2);
    assert_eq!(details.period(), Some((2026, 2)));
    assert_eq!(ids(&details), vec![1, 2]);
    let pool = details.pool().read().unwrap();
    assert_eq!(pool.estimated_median(1), Some(1_000_000.0 / 10.0));
}

#[tokio::test]
async fn later_in_the_month_uses_the_current_month() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 20)]).await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .and(query_param_is_missing("year"))
        .and(query_param_is_missing("month"))
        .respond_with(detail(1))
        .expect(1)
        .mount(&server)
        .await;

    let (scan, details) = run(&server, &config()).await;
    assert_eq!(scan.unwrap().month.as_deref(), Some("2026-03"));
    assert_eq!(details.period(), None);
}

fn ranked_page(rank_for: impl Fn(u32) -> i32 + Send + Sync + 'static) -> impl wiremock::Respond {
    move |request: &wiremock::Request| {
        let page: u32 = request
            .url
            .query_pairs()
            .find(|(k, _)| k == "page")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(0);
        let rank = rank_for(page);
        page_of(vec![listed(u64::from(page) + 1, rank, 20)], 1)
    }
}

fn page_of(circles: Vec<Value>, limit: u32) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circles": circles,
        "total": 0,
        "page": 0,
        "limit": limit,
        "total_pages": 0,
    }))
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
async fn lists_past_a_hundred_pages_when_the_window_needs_it() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ranked_page(|page| page as i32 + 1))
        .mount(&server)
        .await;
    let config = ScanConfig {
        rank_from: 1,
        rank_to: 150,
        max_details: 0,
        page_size: 1,
        lower_to: None,
    };

    let (result, details) = run(&server, &config).await;

    assert_eq!(result.unwrap().candidates, 150);
    assert_eq!(details.pool().read().unwrap().len(), 150);
    assert_eq!(list_calls(&server).await, 150);
}

#[tokio::test]
async fn stops_at_the_page_cap_when_pages_never_end() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ranked_page(|_| 1))
        .mount(&server)
        .await;
    let config = ScanConfig {
        rank_from: 1,
        rank_to: 5,
        max_details: 0,
        page_size: 1,
        lower_to: None,
    };

    run(&server, &config).await.0.unwrap();

    assert_eq!(list_calls(&server).await, 6);
}

fn lower_config() -> ScanConfig {
    ScanConfig {
        rank_from: 1,
        rank_to: 3,
        max_details: 10,
        page_size: 3,
        lower_to: Some(12),
    }
}

async fn mount_lower_page(server: &MockServer, n: u32, circles: Vec<Value>, times: u64) {
    Mock::given(path("/api/v4/circles/list"))
        .and(query_param("page", n.to_string()))
        .and(query_param("max_rank", "12"))
        .respond_with(page(circles))
        .expect(times)
        .mount(server)
        .await;
}

fn without_fans(id: u64, rank: i32) -> Value {
    let mut row = listed(id, rank, 20);
    row["monthly_point"] = json!(0);
    row
}

fn stalled(id: u64, rank: i32) -> Value {
    let mut row = listed(id, rank, 20);
    row["yesterday_points"] = row["monthly_point"].clone();
    row
}

#[tokio::test]
async fn the_lower_range_stops_at_a_page_ending_without_fans() {
    let server = MockServer::start().await;
    mount_lower_page(
        &server,
        0,
        vec![listed(1, 1, 20), listed(2, 2, 20), listed(3, 3, 20)],
        1,
    )
    .await;
    mount_lower_page(
        &server,
        1,
        vec![listed(4, 4, 20), listed(5, 5, 20), without_fans(6, 6)],
        1,
    )
    .await;
    mount_lower_page(&server, 2, vec![listed(7, 7, 20)], 0).await;

    let (scan, _) = run(&server, &lower_config()).await;
    assert_eq!(scan.unwrap().candidates, 5);
}

#[tokio::test]
async fn the_lower_range_stops_at_its_last_rank() {
    let server = MockServer::start().await;
    for n in 0..4 {
        let rows = (1..=3)
            .map(|i| {
                let id = u64::from(n * 3 + i);
                listed(id, id as i32, 20)
            })
            .collect();
        mount_lower_page(&server, n, rows, 1).await;
    }
    mount_lower_page(&server, 4, vec![listed(13, 13, 20)], 0).await;

    let (scan, _) = run(&server, &lower_config()).await;
    assert_eq!(scan.unwrap().candidates, 12);
}

#[tokio::test]
async fn the_lower_range_keeps_only_clubs_still_gaining_fans() {
    let server = MockServer::start().await;
    mount_lower_page(
        &server,
        0,
        vec![stalled(1, 1), listed(2, 2, 20), listed(3, 3, 20)],
        1,
    )
    .await;
    mount_lower_page(&server, 1, vec![stalled(4, 4), listed(5, 5, 20)], 1).await;
    for id in [1, 2, 3] {
        mount_detail(&server, id, detail(id), 1).await;
    }
    for id in [4, 5] {
        mount_detail(&server, id, detail(id), 0).await;
    }

    let (scan, details) = run(&server, &lower_config()).await;
    assert_eq!(scan.unwrap().candidates, 4);
    assert_eq!(ids(&details), vec![1, 2, 3]);
    assert!(!details.pool().read().unwrap().contains(4));
    assert!(details.pool().read().unwrap().contains(5));
}
