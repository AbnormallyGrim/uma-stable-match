//! Trainer pace lookup tests against a mock server. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use uma_stable_match_server::{
    router, run_scheduler, AppState, Cache, ClubDetails, DetailConfig, MatchSettings, PaceLookup,
    RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TRAINER: u64 = 4242;

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

fn series(pace: i64) -> Vec<i64> {
    let mut fans = vec![0i64; 31];
    for (day, value) in fans.iter_mut().take(10).enumerate() {
        *value = 100_000_000 + pace * day as i64;
    }
    fans
}

fn club_detail() -> Value {
    let members: Vec<Value> = (0..10)
        .map(|i| json!({ "viewer_id": 9000 + i, "year": 2026, "month": 3, "daily_fans": series(4_000_000 + 500_000 * i) }))
        .collect();
    json!({
        "circle": {
            "circle_id": 1,
            "name": "Pace Club",
            "member_count": 10,
            "join_style": 1,
            "monthly_rank": 100,
            "monthly_point": 40_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        },
        "members": members,
    })
}

fn trainer_club(daily_fans: Vec<i64>) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": { "circle_id": 2, "name": "Home Club" },
        "members": [
            { "viewer_id": 1, "year": 2026, "month": 3, "daily_fans": series(1_000_000) },
            { "viewer_id": TRAINER, "year": 2026, "month": 3, "daily_fans": daily_fans },
        ],
    }))
}

async fn mount_trainer(server: &MockServer, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn unscanned(server: &MockServer) -> (Cache, ClubDetails) {
    (Cache::new(), details(server))
}

fn details(server: &MockServer) -> ClubDetails {
    ClubDetails::new(
        &client(server),
        SharedPool::default(),
        DetailConfig::default(),
    )
    .0
}

async fn scanned(server: &MockServer) -> (Cache, ClubDetails) {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "circles": [club_detail()["circle"].clone()] })),
        )
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(club_detail()))
        .mount(server)
        .await;

    let cache = Cache::new();
    let details = details(server);
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server),
        details.clone(),
        ScanConfig::default(),
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
    (cache, details)
}

fn app(server: &MockServer, (cache, details): (Cache, ClubDetails), ttl: Duration) -> AppState {
    AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(client(server), ttl),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

async fn get(state: &AppState, query: &str) -> (StatusCode, Value) {
    let response = router(state.clone())
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

fn by_trainer() -> String {
    format!("?trainer_id={TRAINER}")
}

#[tokio::test]
async fn works_out_pace_from_trainer_id() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(&server, trainer_club(series(6_000_000)), 1).await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    let (status, body) = get(&state, &by_trainer()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["trainer"]["pace_per_day"].as_f64().unwrap(),
        6_000_000.0
    );
    assert_eq!(body["trainer"]["trainer_id"], TRAINER);
    assert_eq!(body["clubs"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn caches_the_pace() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(&server, trainer_club(series(6_000_000)), 1).await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    for _ in 0..3 {
        let (status, _) = get(&state, &by_trainer()).await;
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn cached_pace_expires() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(&server, trainer_club(series(6_000_000)), 2).await;
    let state = app(&server, cache, Duration::from_millis(50));

    get(&state, &by_trainer()).await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    get(&state, &by_trainer()).await;
}

#[tokio::test]
async fn unknown_trainer_is_not_found() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(&server, ResponseTemplate::new(404), 1).await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    for _ in 0..2 {
        let (status, body) = get(&state, &by_trainer()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "trainer not found");
    }
}

#[tokio::test]
async fn trainer_missing_from_their_club_is_not_found() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "circle": { "circle_id": 2 }, "members": [] })),
        1,
    )
    .await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    let (status, _) = get(&state, &by_trainer()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn too_little_data_is_unprocessable() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    let mut fans = vec![0i64; 31];
    fans[0] = 100_000_000;
    fans[1] = 105_000_000;
    mount_trainer(&server, trainer_club(fans), 1).await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    let (status, body) = get(&state, &by_trainer()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["error"],
        "not enough fan data this month to work out a pace"
    );
}

#[tokio::test]
async fn upstream_failures_are_bad_gateway_and_not_cached() {
    let server = MockServer::start().await;
    let cache = scanned(&server).await;
    mount_trainer(&server, ResponseTemplate::new(500), 2).await;
    let state = app(&server, cache, DEFAULT_PACE_TTL);

    for _ in 0..2 {
        let (status, body) = get(&state, &by_trainer()).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"], "could not reach uma.moe");
    }
}

#[tokio::test]
async fn no_lookup_before_the_first_scan() {
    let server = MockServer::start().await;
    mount_trainer(&server, trainer_club(series(6_000_000)), 0).await;
    let state = app(&server, unscanned(&server), DEFAULT_PACE_TTL);

    let (status, _) = get(&state, &by_trainer()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn rejects_invalid_trainer_queries() {
    let server = MockServer::start().await;
    let state = app(&server, unscanned(&server), DEFAULT_PACE_TTL);
    let cases = [
        (
            format!("?pace=5000000&trainer_id={TRAINER}"),
            "use either pace or trainer_id, not both",
        ),
        (
            "?trainer_id=abc".to_owned(),
            "trainer_id must be a trainer ID",
        ),
        (
            "?trainer_id=-1".to_owned(),
            "trainer_id must be a trainer ID",
        ),
        ("?trainer_id=".to_owned(), "pace or trainer_id is required"),
    ];
    for (query, message) in cases {
        let (status, body) = get(&state, &query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(body["error"], message, "{query}");
    }
}
