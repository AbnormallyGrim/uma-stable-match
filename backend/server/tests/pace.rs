//! Trainer pace lookup tests against a mock server. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use uma_stable_match_server::{
    router, run_scheduler, AppState, Cache, ClubDetails, DetailConfig, MatchSettings, PaceError,
    PaceLookup, RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param, query_param_is_missing};
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
        Some(Duration::from_secs(3600)),
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

const FEBRUARY: Option<(i32, u32)> = Some((2026, 2));

async fn mount_last_month(server: &MockServer, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .and(query_param("year", "2026"))
        .and(query_param("month", "2"))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

async fn mount_gains(server: &MockServer, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/rankings/gains"))
        .and(query_param("query", TRAINER.to_string()))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn gains(gain_7d: Option<i64>) -> ResponseTemplate {
    let mut rankings = vec![json!({ "viewer_id": 1, "gain_7d": 99_000_000 })];
    if let Some(gain) = gain_7d {
        rankings.push(json!({ "viewer_id": TRAINER, "gain_7d": gain }));
    }
    ResponseTemplate::new(200).set_body_json(json!({ "rankings": rankings }))
}

fn one_day() -> Vec<i64> {
    let mut fans = vec![0; 31];
    fans[0] = 100_000_000;
    fans
}

fn lookup(server: &MockServer) -> PaceLookup {
    PaceLookup::new(client(server), DEFAULT_PACE_TTL)
}

#[tokio::test]
async fn early_month_uses_last_months_fans() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(series(3_000_000)), 1).await;
    mount_gains(&server, gains(Some(70_000_000)), 0).await;

    let pace = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn early_month_falls_back_to_seven_day_gains() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(one_day()), 1).await;
    mount_gains(&server, gains(Some(70_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap();
    assert_eq!(pace, 10_000_000.0);
}

#[tokio::test]
async fn early_month_without_gains_is_not_enough_data() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(one_day()), 1).await;
    mount_gains(&server, gains(None), 1).await;

    let error = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap_err();
    assert!(matches!(error, PaceError::NotEnoughData));
}

#[tokio::test]
async fn early_month_ignores_zero_gains() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(one_day()), 1).await;
    mount_gains(&server, gains(Some(0)), 1).await;

    let error = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap_err();
    assert!(matches!(error, PaceError::NotEnoughData));
}

#[tokio::test]
async fn early_month_unknown_trainer_is_not_found() {
    let server = MockServer::start().await;
    mount_last_month(&server, ResponseTemplate::new(404), 1).await;
    mount_gains(&server, ResponseTemplate::new(404), 1).await;

    let error = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap_err();
    assert!(matches!(error, PaceError::NotFound));
}

#[tokio::test]
async fn early_month_gains_failure_is_upstream() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(one_day()), 1).await;
    mount_gains(&server, ResponseTemplate::new(500), 1).await;

    let error = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap_err();
    assert!(matches!(error, PaceError::Upstream(_)));
}

#[tokio::test]
async fn paces_are_cached_per_month() {
    let server = MockServer::start().await;
    mount_last_month(&server, trainer_club(series(3_000_000)), 1).await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .and(query_param_is_missing("year"))
        .respond_with(trainer_club(series(5_000_000)))
        .expect(1)
        .mount(&server)
        .await;

    let paces = lookup(&server);
    for _ in 0..2 {
        assert_eq!(paces.pace(TRAINER, None).await.unwrap(), 5_000_000.0);
        assert_eq!(paces.pace(TRAINER, FEBRUARY).await.unwrap(), 3_000_000.0);
    }
    assert!(paces.is_cached(TRAINER, None));
    assert!(paces.is_cached(TRAINER, FEBRUARY));
    assert!(!paces.is_cached(TRAINER, Some((2026, 1))));
}

#[tokio::test]
async fn early_month_recommendations_use_last_month() {
    let server = MockServer::start().await;
    let mut row = club_detail()["circle"].clone();
    row["last_updated"] = json!("2026-03-02T15:00:00Z");
    row["last_month_point"] = json!(1_120_000_000);
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": [row] })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .and(query_param("month", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(club_detail()))
        .expect(1)
        .mount(&server)
        .await;
    mount_last_month(&server, trainer_club(series(3_000_000)), 1).await;

    let cache = Cache::new();
    let details = details(&server);
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(&server),
        details.clone(),
        ScanConfig::default(),
        Some(Duration::from_secs(3600)),
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

    let state = app(&server, (cache, details), DEFAULT_PACE_TTL);
    let (status, body) = get(&state, &by_trainer()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["month"], "2026-02");
    assert_eq!(body["trainer"]["pace_per_day"], 3_000_000.0);
    assert_eq!(body["total"], 1);
}

async fn mount_rankings(server: &MockServer, rows: Vec<Value>, times: u64) {
    Mock::given(path("/api/v4/rankings/monthly"))
        .and(query_param("query", TRAINER.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "rankings": rows })))
        .expect(times)
        .mount(server)
        .await;
}

async fn mount_club(server: &MockServer, circle_id: u64, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", circle_id.to_string()))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn ranked_in(circle_id: Option<u64>) -> Value {
    json!({ "viewer_id": TRAINER, "circle_id": circle_id })
}

fn oldest_club() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "circle": { "circle_id": 7, "name": "First Club" },
        "members": [{ "viewer_id": 1, "year": 2026, "month": 3, "daily_fans": series(1_000_000) }],
    }))
}

#[tokio::test]
async fn a_trainer_who_changed_clubs_is_found_in_their_ranked_club() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![ranked_in(Some(2))], 1).await;
    mount_club(&server, 2, trainer_club(series(3_000_000)), 1).await;
    mount_trainer(&server, oldest_club(), 0).await;

    let pace = lookup(&server).pace(TRAINER, None).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn only_the_exact_trainer_id_counts_in_rankings() {
    let server = MockServer::start().await;
    let partial_match = json!({ "viewer_id": TRAINER + 1, "circle_id": 3 });
    mount_rankings(&server, vec![partial_match], 1).await;
    mount_club(&server, 3, trainer_club(series(9_000_000)), 0).await;
    mount_trainer(&server, trainer_club(series(3_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, None).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn without_a_rankings_row_the_trainer_lookup_is_used() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![], 1).await;
    mount_trainer(&server, trainer_club(series(3_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, None).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn a_ranked_trainer_without_a_club_falls_back() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![ranked_in(None)], 1).await;
    mount_trainer(&server, ResponseTemplate::new(404), 1).await;

    let error = lookup(&server).pace(TRAINER, None).await.unwrap_err();
    assert!(matches!(error, PaceError::NotFound));
}

#[tokio::test]
async fn a_rankings_failure_falls_back() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/rankings/monthly"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;
    mount_trainer(&server, trainer_club(series(3_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, None).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn a_ranked_club_without_the_trainer_falls_back() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![ranked_in(Some(7))], 1).await;
    mount_club(&server, 7, oldest_club(), 1).await;
    mount_trainer(&server, trainer_club(series(3_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, None).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn too_little_data_in_the_ranked_club_is_not_enough_data() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![ranked_in(Some(2))], 1).await;
    mount_club(&server, 2, trainer_club(one_day()), 1).await;
    mount_trainer(&server, oldest_club(), 0).await;

    let error = lookup(&server).pace(TRAINER, None).await.unwrap_err();
    assert!(matches!(error, PaceError::NotEnoughData));
}

#[tokio::test]
async fn early_month_finds_last_months_club_through_rankings() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/rankings/monthly"))
        .and(query_param("query", TRAINER.to_string()))
        .and(query_param("year", "2026"))
        .and(query_param("month", "2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "rankings": [ranked_in(Some(2))] })),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "2"))
        .and(query_param("month", "2"))
        .respond_with(trainer_club(series(3_000_000)))
        .expect(1)
        .mount(&server)
        .await;
    mount_last_month(&server, oldest_club(), 0).await;
    mount_gains(&server, gains(Some(70_000_000)), 0).await;

    let pace = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap();
    assert_eq!(pace, 3_000_000.0);
}

#[tokio::test]
async fn early_month_ranked_club_with_too_little_data_uses_gains() {
    let server = MockServer::start().await;
    mount_rankings(&server, vec![ranked_in(Some(2))], 1).await;
    mount_club(&server, 2, trainer_club(one_day()), 1).await;
    mount_last_month(&server, oldest_club(), 0).await;
    mount_gains(&server, gains(Some(70_000_000)), 1).await;

    let pace = lookup(&server).pace(TRAINER, FEBRUARY).await.unwrap();
    assert_eq!(pace, 10_000_000.0);
}

#[tokio::test]
async fn searches_read_the_ranked_club_from_the_club_cache() {
    let server = MockServer::start().await;
    let (cache, details) = scanned(&server).await;
    let member = 9003;
    Mock::given(path("/api/v4/rankings/monthly"))
        .and(query_param("query", member.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "rankings": [{ "viewer_id": member, "circle_id": 1 }] })),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", member.to_string()))
        .respond_with(oldest_club())
        .expect(0)
        .mount(&server)
        .await;
    let state = AppState {
        paces: PaceLookup::new(client(&server), DEFAULT_PACE_TTL).with_details(details.clone()),
        ..app(&server, (cache, details), DEFAULT_PACE_TTL)
    };

    let (status, body) = get(&state, &format!("?trainer_id={member}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["trainer"]["pace_per_day"], 5_500_000.0);
}
