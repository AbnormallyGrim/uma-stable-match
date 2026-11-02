//! Club page route tests against a mock server. All data is made up.

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
const M: f64 = 1_000_000.0;

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

// Ten members from 4M to 8.5M a day, plus the trainer at 6M if `with_trainer`.
fn club_detail(with_trainer: bool) -> Value {
    let mut members: Vec<Value> = (0..10)
        .map(|i| json!({ "viewer_id": 9000 + i, "trainer_name": format!("Made-up Trainer {i}"), "year": 2026, "month": 3, "daily_fans": series(4_000_000 + 500_000 * i) }))
        .collect();
    if with_trainer {
        members.push(
            json!({ "viewer_id": TRAINER, "year": 2026, "month": 3, "daily_fans": series(6_000_000) }),
        );
    }
    json!({
        "circle": {
            "circle_id": 1,
            "name": "Page Club",
            "member_count": members.len(),
            "join_style": 2,
            "policy": 2,
            "monthly_rank": 100,
            "last_month_rank": 120,
            "monthly_point": 40_000_000,
            "comment": "Made-up club, join our Discord",
            "last_updated": "2026-03-19T15:00:00Z",
        },
        "members": members,
    })
}

async fn mount_trainer(server: &MockServer, club: Value) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(club))
        .mount(server)
        .await;
}

fn home_club() -> Value {
    json!({
        "circle": { "circle_id": 2, "name": "Home Club" },
        "members": [
            { "viewer_id": TRAINER, "year": 2026, "month": 3, "daily_fans": series(6_000_000) },
        ],
    })
}

async fn scanned(server: &MockServer, detail: Value) -> AppState {
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "circles": [detail["circle"]] })),
        )
        .mount(server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .expect(1)
        .mount(server)
        .await;
    // Clubs that aren't loaded are never fetched for the page.
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(0)
        .mount(server)
        .await;

    let cache = Cache::new();
    let details = ClubDetails::new(
        client(server),
        SharedPool::default(),
        DetailConfig::default(),
    )
    .0;
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
    AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client(server), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

async fn get(state: &AppState, uri: &str) -> (StatusCode, Value) {
    let response = router(state.clone())
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

fn paces(body: &Value) -> Vec<f64> {
    body["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["pace_per_day"].as_f64().unwrap())
        .collect()
}

#[tokio::test]
async fn shows_a_loaded_clubs_members_fastest_first() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;

    let (status, body) = get(&state, "/api/v1/clubs/1").await;
    assert_eq!(status, StatusCode::OK);
    let club = &body["club"];
    assert_eq!(club["circle_id"], 1);
    assert_eq!(club["name"], "Page Club");
    assert_eq!(club["url"], "https://uma.moe/circles/1");
    assert_eq!(club["join_style"], "approval");
    assert_eq!(club["member_count"], 10);
    assert_eq!(club["open_spots"], 20);
    assert_eq!(club["monthly_rank"], 100);
    assert_eq!(club["has_discord"], true);
    assert_eq!(body["month"], "2026-03");
    assert_eq!(body["updated_at"], "2026-03-19T15:00:00Z");
    assert_eq!(body["median_per_day"].as_f64().unwrap(), 6.25 * M);
    let expected: Vec<f64> = (0..10)
        .rev()
        .map(|i| 4.0 * M + 0.5 * M * i as f64)
        .collect();
    assert_eq!(paces(&body), expected);
    assert_eq!(body["members"][0]["daily_fans"], json!(series(8_500_000)));
    assert_eq!(body["members"][0]["name"], "Made-up Trainer 9");
    assert_eq!(body["members"][9]["name"], "Made-up Trainer 0");
    assert_eq!(body["trainer"], Value::Null);
}

#[tokio::test]
async fn places_the_trainer_among_the_members() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;
    mount_trainer(&server, home_club()).await;

    let (status, body) = get(&state, &format!("/api/v1/clubs/1?trainer_id={TRAINER}")).await;
    assert_eq!(status, StatusCode::OK);
    let trainer = &body["trainer"];
    assert_eq!(trainer["trainer_id"], TRAINER);
    assert_eq!(trainer["pace_per_day"].as_f64().unwrap(), 6.0 * M);
    assert_eq!(trainer["daily_fans"], json!(series(6_000_000)));
    assert_eq!(trainer["in_club"], false);
    assert_eq!(trainer["your_place"], 6);
    assert_eq!(trainer["club_size"], 11);
}

#[tokio::test]
async fn leaves_the_trainers_own_row_out_of_their_club() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(true)).await;
    mount_trainer(&server, club_detail(true)).await;

    let (status, body) = get(&state, &format!("/api/v1/clubs/1?trainer_id={TRAINER}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["members"].as_array().unwrap().len(), 10);
    assert_eq!(body["club"]["member_count"], 11);
    assert_eq!(body["median_per_day"].as_f64().unwrap(), 6.25 * M);
    assert_eq!(body["trainer"]["in_club"], true);
    assert_eq!(body["trainer"]["your_place"], 6);
    assert_eq!(body["trainer"]["club_size"], 11);
}

#[tokio::test]
async fn a_club_that_isnt_loaded_is_not_found_and_not_fetched() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;

    let (status, body) = get(&state, "/api/v1/clubs/99").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "club not loaded yet; try again later");
}

#[tokio::test]
async fn an_unknown_trainer_is_not_found() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let (status, body) = get(&state, &format!("/api/v1/clubs/1?trainer_id={TRAINER}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "trainer not found");
}

#[tokio::test]
async fn rejects_invalid_input() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;
    let cases = [
        ("/api/v1/clubs/abc", "club ID must be a club ID"),
        ("/api/v1/clubs/-1", "club ID must be a club ID"),
        (
            "/api/v1/clubs/1?trainer_id=me",
            "trainer_id must be a trainer ID",
        ),
        (
            "/api/v1/clubs/1?trainer_id=1&trainer_id=2",
            "invalid query string",
        ),
    ];
    for (uri, message) in cases {
        let (status, body) = get(&state, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(body["error"], message, "{uri}");
    }
}

#[tokio::test]
async fn a_blank_trainer_id_shows_the_club_alone() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;

    let (status, body) = get(&state, "/api/v1/clubs/1?trainer_id=").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["trainer"], Value::Null);
}

// Early in the month: ten members with only 2 days of data, from 4M to 8.5M a day.
fn early_detail() -> Value {
    let mut detail = club_detail(false);
    for member in detail["members"].as_array_mut().unwrap() {
        let fans = member["daily_fans"].as_array_mut().unwrap();
        for day in fans.iter_mut().skip(2) {
            *day = json!(0);
        }
    }
    detail
}

// Last month's details for club 1: members 9000 to 9007, who are still in it, and one who left.
fn previous_detail() -> Value {
    let mut members: Vec<Value> = (0..8)
        .map(|i| json!({ "viewer_id": 9000 + i, "year": 2026, "month": 2, "daily_fans": series(3_000_000 + 100_000 * i) }))
        .collect();
    members.push(
        json!({ "viewer_id": 9100, "year": 2026, "month": 2, "daily_fans": series(1_000_000) }),
    );
    json!({ "circle": { "circle_id": 1, "name": "Page Club" }, "members": members })
}

async fn mount_previous(server: &MockServer, response: ResponseTemplate, times: u64) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .and(query_param("year", "2026"))
        .and(query_param("month", "2"))
        .respond_with(response)
        .with_priority(1)
        .expect(times)
        .mount(server)
        .await;
}

async fn mount_previous_trainer(server: &MockServer) {
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", TRAINER.to_string()))
        .and(query_param("month", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": { "circle_id": 2, "name": "Home Club" },
            "members": [{ "viewer_id": TRAINER, "year": 2026, "month": 2, "daily_fans": series(5_000_000) }],
        })))
        .with_priority(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn early_in_the_month_it_adds_last_months_fans_once() {
    let server = MockServer::start().await;
    let state = scanned(&server, early_detail()).await;
    mount_previous(
        &server,
        ResponseTemplate::new(200).set_body_json(previous_detail()),
        1,
    )
    .await;
    mount_trainer(&server, home_club()).await;
    mount_previous_trainer(&server).await;

    let uri = format!("/api/v1/clubs/1?trainer_id={TRAINER}");
    let (status, body) = get(&state, &uri).await;
    assert_eq!(status, StatusCode::OK);
    let previous = &body["previous"];
    assert_eq!(previous["month"], "2026-02");
    // No member has a pace from 2 days without a prior, so they keep their listed order. Members
    // 9008 and 9009 joined this month, so they have no line.
    let members = previous["members"].as_array().unwrap();
    assert_eq!(members.len(), body["members"].as_array().unwrap().len());
    assert_eq!(members[0], json!(series(3_000_000)));
    assert_eq!(members[7], json!(series(3_700_000)));
    assert_eq!(members[8], Value::Null);
    assert_eq!(members[9], Value::Null);
    assert_eq!(previous["trainer"], json!(series(5_000_000)));

    let (status, again) = get(&state, &uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["previous"], body["previous"]);
}

#[tokio::test]
async fn with_3_days_it_still_adds_last_month() {
    let server = MockServer::start().await;
    let mut detail = club_detail(false);
    for member in detail["members"].as_array_mut().unwrap() {
        for day in member["daily_fans"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .skip(3)
        {
            *day = json!(0);
        }
    }
    let state = scanned(&server, detail).await;
    mount_previous(
        &server,
        ResponseTemplate::new(200).set_body_json(previous_detail()),
        1,
    )
    .await;

    let (status, body) = get(&state, "/api/v1/clubs/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["previous"]["month"], "2026-02");
}

#[tokio::test]
async fn with_enough_days_it_fetches_nothing_more() {
    let server = MockServer::start().await;
    let state = scanned(&server, club_detail(false)).await;
    mount_previous(&server, ResponseTemplate::new(200), 0).await;

    let (status, body) = get(&state, "/api/v1/clubs/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["previous"], Value::Null);
}

#[tokio::test]
async fn over_the_lookup_limit_it_shows_this_month_alone() {
    let server = MockServer::start().await;
    let mut state = scanned(&server, early_detail()).await;
    // The trainer lookup takes the one lookup this visitor has.
    state.limits = RateLimits::new(10_000, 1, false);
    mount_previous(&server, ResponseTemplate::new(200), 0).await;
    mount_trainer(&server, home_club()).await;
    mount_previous_trainer(&server).await;

    let (status, body) = get(&state, &format!("/api/v1/clubs/1?trainer_id={TRAINER}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["previous"], Value::Null);
    assert_eq!(body["members"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn a_failed_fetch_shows_this_month_alone_and_tries_again_later() {
    let server = MockServer::start().await;
    let state = scanned(&server, early_detail()).await;
    mount_previous(&server, ResponseTemplate::new(500), 2).await;

    for _ in 0..2 {
        let (status, body) = get(&state, "/api/v1/clubs/1").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["previous"], Value::Null);
    }
}
