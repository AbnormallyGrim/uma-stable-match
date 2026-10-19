use std::time::Duration;

use serde_json::{json, Value};
use uma_stable_match_server::{scan, ScanConfig, ScanError};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{method, path, query_param};
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
    }
}

fn listed(id: u64, rank: i32, last_rank: i32, members: i32) -> Value {
    json!({
        "circle_id": id,
        "name": format!("Scan Club {id}"),
        "monthly_rank": rank,
        "last_month_rank": last_rank,
        "member_count": members,
        "join_style": 1,
    })
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

fn ids(scan: &uma_stable_match_server::Scan) -> Vec<u64> {
    scan.clubs.iter().map(|c| c.circle_id).collect()
}

#[tokio::test]
async fn pages_until_past_the_window() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![
            listed(1, 5, 5, 20),
            listed(2, 12, 12, 20),
            listed(3, 20, 20, 20),
        ],
    )
    .await;
    mount_page(
        &server,
        1,
        vec![
            listed(4, 30, 30, 20),
            listed(5, 40, 40, 20),
            listed(6, 50, 50, 20),
        ],
    )
    .await;
    for id in 2..=6 {
        mount_detail(&server, id, detail(id), 1).await;
    }

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(scan.candidates, 5);
    assert_eq!(scan.failed, 0);
    assert_eq!(ids(&scan), vec![2, 3, 4, 5, 6]);
}

#[tokio::test]
async fn stops_on_a_short_page() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 12, 20), listed(2, 13, 13, 20)],
    )
    .await;
    mount_detail(&server, 1, detail(1), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(ids(&scan), vec![1, 2]);
}

#[tokio::test]
async fn stops_on_an_empty_page() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![
            listed(1, 12, 12, 20),
            listed(2, 13, 13, 20),
            listed(3, 14, 14, 20),
        ],
    )
    .await;
    mount_page(&server, 1, vec![]).await;
    for id in 1..=3 {
        mount_detail(&server, id, detail(id), 1).await;
    }

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(scan.candidates, 3);
}

#[tokio::test]
async fn skips_full_clubs_and_unknown_sizes() {
    let server = MockServer::start().await;
    let mut unknown = listed(3, 14, 14, 20);
    unknown["member_count"] = Value::Null;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 12, 30), listed(2, 13, 13, 29), unknown],
    )
    .await;
    mount_page(&server, 1, vec![]).await;
    mount_detail(&server, 1, detail(1), 0).await;
    mount_detail(&server, 2, detail(2), 1).await;
    mount_detail(&server, 3, detail(3), 0).await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(ids(&scan), vec![2]);
}

#[tokio::test]
async fn fetches_climbers_first_up_to_the_cap() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![
            listed(1, 12, 13, 20),
            listed(2, 13, 40, 20),
            listed(3, 14, 30, 20),
        ],
    )
    .await;
    mount_page(
        &server,
        1,
        vec![listed(4, 15, 15, 20), listed(5, 16, 10, 20)],
    )
    .await;
    mount_detail(&server, 2, detail(2), 1).await;
    mount_detail(&server, 3, detail(3), 1).await;
    for id in [1, 4, 5] {
        mount_detail(&server, id, detail(id), 0).await;
    }

    let mut config = config();
    config.max_details = 2;
    let scan = scan(&client(&server), &config).await.unwrap();
    assert_eq!(scan.candidates, 5);
    assert_eq!(ids(&scan), vec![2, 3]);
}

#[tokio::test]
async fn equal_climbs_keep_list_order() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 22, 20), listed(2, 13, 23, 20)],
    )
    .await;
    mount_detail(&server, 1, detail(1), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(ids(&scan), vec![1, 2]);
}

#[tokio::test]
async fn skips_clubs_that_fail_to_load() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        vec![listed(1, 12, 12, 20), listed(2, 13, 13, 20)],
    )
    .await;
    mount_detail(&server, 1, ResponseTemplate::new(500), 1).await;
    mount_detail(&server, 2, detail(2), 1).await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    assert_eq!(scan.failed, 1);
    assert_eq!(ids(&scan), vec![2]);
}

#[tokio::test]
async fn converts_details_to_clubs() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 12, 20)]).await;
    mount_detail(&server, 1, detail(1), 1).await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    let club = &scan.clubs[0];
    assert_eq!(club.club_rank, Some(5));
    assert_eq!(club.members.len(), 1);
    assert_eq!(club.members[0].daily_fans, vec![100, 200, 300]);
}

#[tokio::test]
async fn falls_back_to_listed_details_when_missing() {
    let server = MockServer::start().await;
    mount_page(&server, 0, vec![listed(1, 12, 12, 20)]).await;
    mount_detail(
        &server,
        1,
        ResponseTemplate::new(200).set_body_json(json!({ "members": [], "club_rank": 4 })),
        1,
    )
    .await;

    let scan = scan(&client(&server), &config()).await.unwrap();
    let club = &scan.clubs[0];
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

    let error = scan(&client(&server), &config()).await.unwrap_err();
    assert!(matches!(error, ScanError::List(_)));
}
