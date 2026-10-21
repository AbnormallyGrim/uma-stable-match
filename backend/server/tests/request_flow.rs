//! Shortlist, fetch budget and coverage tests against a mock server. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use uma_stable_match_server::{
    router, run_scheduler, AppState, BackgroundLoader, Cache, ClubDetails, DetailConfig,
    MatchSettings, PaceLookup, RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const M: i64 = 1_000_000;
const CLUBS: u64 = 6;

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

fn circle(id: u64, join_style: i32) -> Value {
    json!({
        "circle_id": id,
        "name": format!("Flow Club {id}"),
        "member_count": 10,
        "join_style": join_style,
        "monthly_rank": 100 + id,
        "monthly_point": id as i64 * M * 18 * 10,
        "last_updated": "2026-03-19T15:00:00Z",
    })
}

fn detail(id: u64, join_style: i32) -> Value {
    let members: Vec<Value> = (0..10)
        .map(|k| {
            let gain = id as i64 * M + k * M / 10;
            let fans: Vec<i64> = (0..=18).map(|day| 100 * M + gain * day).collect();
            json!({ "viewer_id": 7000 + id * 100 + k as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    json!({ "circle": circle(id, join_style), "members": members })
}

struct Flow {
    server: MockServer,
    cache: Cache,
    details: ClubDetails,
    loader: Option<BackgroundLoader>,
}

async fn flow(closed: &[u64], failing: &[u64], max_details: u32) -> Flow {
    let server = MockServer::start().await;
    let style = |id: u64| if closed.contains(&id) { 3 } else { 1 };
    let rows: Vec<Value> = (1..=CLUBS).map(|id| circle(id, style(id))).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": rows })))
        .mount(&server)
        .await;
    for id in 1..=CLUBS {
        let response = if failing.contains(&id) {
            ResponseTemplate::new(500)
        } else {
            ResponseTemplate::new(200).set_body_json(detail(id, style(id)))
        };
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .respond_with(response)
            .mount(&server)
            .await;
    }

    let (details, loader) = ClubDetails::new(
        &client(&server),
        SharedPool::default(),
        DetailConfig::default(),
    );
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let config = ScanConfig {
        rank_from: 1,
        rank_to: 900,
        max_details,
        page_size: 100,
    };
    let handle = tokio::spawn(run_scheduler(
        client(&server),
        details.clone(),
        config,
        Duration::from_secs(3600),
        cache.clone(),
        stopped,
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while cache.snapshot().is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("scan did not finish");
    stop.send(true).unwrap();
    handle.await.unwrap();
    Flow {
        server,
        cache,
        details,
        loader: Some(loader),
    }
}

impl Flow {
    async fn get(&self, matching: MatchSettings, query: &str) -> (StatusCode, Value) {
        let state = AppState {
            cache: self.cache.clone(),
            details: self.details.clone(),
            matching,
            paces: PaceLookup::new(client(&self.server), DEFAULT_PACE_TTL),
            limits: RateLimits::new(10_000, 10_000, false),
        };
        let response = router(state)
            .oneshot(
                Request::get(format!("/api/v1/recommendations{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    async fn fetches(&self, id: u64) -> usize {
        let query = format!("circle_id={id}");
        self.server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/api/v4/circles" && r.url.query() == Some(&query))
            .count()
    }
}

fn settings(shortlist_size: usize, fetch_budget: usize) -> MatchSettings {
    MatchSettings {
        shortlist_size,
        fetch_budget,
    }
}

const NEAR_3M: &str = "?pace=3529412";

fn ids(body: &Value) -> Vec<u64> {
    let mut ids: Vec<u64> = body["clubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["circle_id"].as_u64().unwrap())
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn fetches_the_nearest_clubs_within_the_budget() {
    let flow = flow(&[], &[], 0).await;
    let (status, body) = flow.get(settings(4, 2), NEAR_3M).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["coverage"],
        json!({ "shortlisted": 4, "scored": 2, "pending": 2 })
    );
    assert_eq!(ids(&body), vec![3, 4]);
    for (id, expected) in [(1, 0), (2, 0), (3, 1), (4, 1), (5, 0), (6, 0)] {
        assert_eq!(flow.fetches(id).await, expected, "club {id}");
    }
    assert_eq!(flow.details.queued(), 2);
}

#[tokio::test]
async fn queued_clubs_load_in_the_background() {
    let mut flow = flow(&[], &[], 0).await;
    let loader = tokio::spawn(flow.loader.take().unwrap().run());
    flow.get(settings(4, 2), NEAR_3M).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while flow.details.queued() > 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("background loads did not finish");

    let (_, body) = flow.get(settings(4, 2), NEAR_3M).await;
    assert_eq!(
        body["coverage"],
        json!({ "shortlisted": 4, "scored": 4, "pending": 0 })
    );
    for id in 2..=5 {
        assert_eq!(flow.fetches(id).await, 1, "club {id}");
    }
    loader.abort();
}

#[tokio::test]
async fn concurrent_requests_share_fetches() {
    let flow = flow(&[], &[], 0).await;
    let (a, b) = tokio::join!(
        flow.get(settings(4, 2), NEAR_3M),
        flow.get(settings(4, 2), NEAR_3M)
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(b.0, StatusCode::OK);
    assert_eq!(flow.fetches(3).await, 1);
    assert_eq!(flow.fetches(4).await, 1);
    assert_eq!(flow.details.queued(), 2);
}

#[tokio::test]
async fn every_cached_club_is_scored() {
    let flow = flow(&[], &[], 10).await;
    let (_, body) = flow.get(settings(1, 2), NEAR_3M).await;
    assert_eq!(
        body["coverage"],
        json!({ "shortlisted": 1, "scored": 6, "pending": 0 })
    );
    assert_eq!(ids(&body), vec![1, 2, 3, 4, 5, 6]);
    for id in 1..=CLUBS {
        assert_eq!(flow.fetches(id).await, 1, "club {id}");
    }
}

#[tokio::test]
async fn closed_clubs_are_shortlisted_only_when_asked() {
    let flow = flow(&[3], &[], 0).await;
    let (_, body) = flow.get(settings(2, 2), NEAR_3M).await;
    assert_eq!(ids(&body), vec![2, 4]);
    assert_eq!(flow.fetches(3).await, 0);

    let (_, body) = flow
        .get(settings(2, 2), &format!("{NEAR_3M}&include_closed=true"))
        .await;
    assert_eq!(ids(&body), vec![2, 3, 4]);
    assert_eq!(flow.fetches(3).await, 1);
}

#[tokio::test]
async fn failed_fetches_are_pending_and_queued() {
    let flow = flow(&[], &[3], 0).await;
    let (status, body) = flow.get(settings(2, 2), NEAR_3M).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["coverage"],
        json!({ "shortlisted": 2, "scored": 1, "pending": 1 })
    );
    assert_eq!(ids(&body), vec![4]);
    assert_eq!(flow.details.queued(), 1);
}

#[tokio::test]
async fn invalid_requests_fetch_nothing() {
    let flow = flow(&[], &[], 0).await;
    for query in ["?pace=0", "?pace=3000000&balance_weight=2"] {
        let (status, _) = flow.get(settings(4, 2), query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
    }
    for id in 1..=CLUBS {
        assert_eq!(flow.fetches(id).await, 0, "club {id}");
    }
    assert_eq!(flow.details.queued(), 0);
}

#[tokio::test]
async fn reports_the_data_month() {
    let flow = flow(&[], &[], 10).await;
    let (_, body) = flow.get(MatchSettings::default(), NEAR_3M).await;
    assert_eq!(body["month"], "2026-03");
}

#[tokio::test]
async fn serves_results_while_the_first_scan_loads() {
    let server = MockServer::start().await;
    let rows: Vec<Value> = (1..=CLUBS).map(|id| circle(id, 1)).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": rows })))
        .mount(&server)
        .await;
    for id in 1..=CLUBS {
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(detail(id, 1))
                    .set_delay(Duration::from_secs(if id == 1 { 0 } else { 30 })),
            )
            .mount(&server)
            .await;
    }
    let (details, _loader) = ClubDetails::new(
        &client(&server),
        SharedPool::default(),
        DetailConfig::default(),
    );
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(&server),
        details.clone(),
        ScanConfig {
            rank_from: 1,
            rank_to: 900,
            max_details: 10,
            page_size: 100,
        },
        Duration::from_secs(3600),
        cache.clone(),
        stopped,
    ));
    let flow = Flow {
        server,
        cache: cache.clone(),
        details,
        loader: None,
    };

    let body = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let (status, body) = flow.get(settings(120, 0), NEAR_3M).await;
            if status == StatusCode::OK && body["total"] == 1 {
                return body;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("no results while the scan was loading");

    assert!(cache.snapshot().is_none());
    assert_eq!(body["month"], "2026-03");
    assert_eq!(ids(&body), vec![1]);
    assert!(body["coverage"]["pending"].as_u64().unwrap() > 0);
    assert!(body["updated_at"].as_str().unwrap().ends_with('Z'));

    stop.send(true).unwrap();
    handle.await.unwrap();
}
