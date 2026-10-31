//! Tests that excluded trainers' searches stay out of search metrics. Each test records into its
//! own local recorder. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde_json::json;
use tower::ServiceExt;
use uma_stable_match_server::{
    router, scan, AppState, Cache, ClubDetails, DetailConfig, ExcludedTrainers, MatchSettings,
    PaceLookup, RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const EXCLUDED: u64 = 111111111111;
const VISITOR: u64 = 222222222222;
const CLUB: u64 = 900000501;

fn fans(pace: i64) -> Vec<i64> {
    (0..10).map(|day| 100_000_000 + pace * day).collect()
}

// A scanned club, and a trainer lookup for each made-up trainer.
async fn state(server: &MockServer) -> AppState {
    let members: Vec<_> = (0..10)
        .map(|i| json!({ "viewer_id": 7000 + i, "year": 2026, "month": 3, "daily_fans": fans(4_000_000) }))
        .collect();
    let circle = json!({
        "circle_id": CLUB,
        "name": "Made-up Club",
        "member_count": 10,
        "join_style": 1,
        "monthly_rank": 500,
        "monthly_point": 20_000_000,
        "last_updated": "2026-03-19T15:00:00Z",
    });
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": [circle] })))
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", CLUB.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "circle": circle, "members": members })),
        )
        .mount(server)
        .await;
    for trainer in [EXCLUDED, VISITOR] {
        Mock::given(path("/api/v4/circles"))
            .and(query_param("viewer_id", trainer.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "circle": { "circle_id": 900000599 },
                "members": [{ "viewer_id": trainer, "year": 2026, "month": 3, "daily_fans": fans(5_000_000) }],
            })))
            .mount(server)
            .await;
    }

    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    let details = ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0;
    let config = ScanConfig {
        rank_from: 1,
        rank_to: 10_000,
        ..ScanConfig::default()
    };
    scan(&client, &details, &config).await.unwrap();
    AppState {
        cache: Cache::new(),
        details,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL)
            .with_excluded(ExcludedTrainers::new([EXCLUDED])),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

async fn get(state: &AppState, uri: String) -> StatusCode {
    router(state.clone())
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

fn recorded(handle: &PrometheusHandle, name: &str) -> bool {
    handle.render().lines().any(|line| line.starts_with(name))
}

#[tokio::test]
async fn an_excluded_trainers_search_is_answered_but_not_counted() {
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    let _local = metrics::set_default_local_recorder(&recorder);
    let server = MockServer::start().await;
    let state = state(&server).await;

    let search = |trainer: u64| format!("/api/v1/recommendations?trainer_id={trainer}");
    assert_eq!(get(&state, search(EXCLUDED)).await, StatusCode::OK);
    assert_eq!(
        get(
            &state,
            format!("/api/v1/clubs/{CLUB}?trainer_id={EXCLUDED}")
        )
        .await,
        StatusCode::OK
    );
    assert!(!recorded(&handle, "pace_lookups_total"));
    assert!(!recorded(&handle, "search_shortlist_worst_rank"));
    assert!(!recorded(&handle, "search_pick_worst_rank"));

    assert_eq!(get(&state, search(VISITOR)).await, StatusCode::OK);
    assert!(recorded(&handle, "pace_lookups_total"));
    assert!(recorded(&handle, "search_shortlist_worst_rank"));
}

#[tokio::test]
async fn pace_searches_are_still_counted() {
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    let _local = metrics::set_default_local_recorder(&recorder);
    let server = MockServer::start().await;
    let state = state(&server).await;

    let status = get(&state, "/api/v1/recommendations?pace=5000000".into()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(recorded(&handle, "search_shortlist_worst_rank"));
}
