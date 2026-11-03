//! Which clubs are marked as taking applications through Discord. All data is made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Extension;
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use uma_stable_match_discord::{ClubStore, SettingsUpdate};
use uma_stable_match_server::{
    router, run_scheduler, AppState, Applications, Cache, ClubDetails, DetailConfig, MatchSettings,
    PaceLookup, RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SERVER: u64 = 880_000_000_001;

async fn store() -> Option<ClubStore> {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    Some(ClubStore::new(conn))
}

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

/// A club of `members` members at `join_style` (1 open, 2 approval, 3 closed) with a made-up
/// description.
fn detail(id: u64, join_style: i32, members: u64) -> Value {
    let rows: Vec<Value> = (0..members)
        .map(|i| {
            json!({
                "viewer_id": id * 100 + i,
                "year": 2026,
                "month": 3,
                "daily_fans": series(5_000_000 + 100_000 * i as i64),
            })
        })
        .collect();
    json!({
        "circle": {
            "circle_id": id,
            "name": format!("Applications Club {id}"),
            "member_count": members,
            "join_style": join_style,
            "policy": 2,
            "monthly_rank": 100,
            "last_month_rank": 110,
            "comment": "Made-up club",
            "monthly_point": 20_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        },
        "members": rows,
        "club_rank": 7,
    })
}

async fn scanned(details: Vec<Value>) -> AppState {
    let server = MockServer::start().await;
    let listed: Vec<Value> = details.iter().map(|d| d["circle"].clone()).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": listed })))
        .mount(&server)
        .await;
    for d in details {
        let id = d["circle"]["circle_id"].as_u64().unwrap();
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(d))
            .mount(&server)
            .await;
    }
    let cache = Cache::new();
    let details = ClubDetails::new(
        client(&server),
        SharedPool::default(),
        DetailConfig::default(),
    )
    .0;
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
    AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client(&server), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

async fn get(state: &AppState, store: Option<&ClubStore>, uri: &str) -> Value {
    let app = router(state.clone());
    let app = match store {
        Some(store) => app.layer(Extension(Applications::new(store.clone()))),
        None => app,
    };
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// The clubs in the answer that are marked, lowest ID first.
fn marked(body: &Value) -> Vec<u64> {
    let mut ids: Vec<u64> = body["clubs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|club| club["applications"] == "discord")
        .map(|club| club["circle_id"].as_u64().unwrap())
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn only_linked_open_joinable_clubs_without_a_problem_are_marked() {
    let Some(store) = store().await else {
        return;
    };
    let (ok, paused, closed, full, broken, unlinked) = (
        910_000_001,
        910_000_002,
        910_000_003,
        910_000_004,
        910_000_005,
        910_000_006,
    );
    let ids = [ok, paused, closed, full, broken, unlinked];
    let state = scanned(vec![
        detail(ok, 2, 10),
        detail(paused, 2, 10),
        detail(closed, 3, 10),
        detail(full, 1, 30),
        detail(broken, 2, 10),
        detail(unlinked, 2, 10),
    ])
    .await;
    for club in &ids[..5] {
        store.link(*club, SERVER).await.unwrap();
    }
    let pause = SettingsUpdate {
        open: Some(false),
        ..SettingsUpdate::default()
    };
    store.update(paused, SERVER, &pause).await.unwrap();
    redis_set(broken, "problem", "made-up problem").await;

    let uri = "/api/v1/recommendations?pace=7600000&include_closed=true&include_full=true&limit=50";
    let body = get(&state, Some(&store), uri).await;
    assert_eq!(body["clubs"].as_array().unwrap().len(), 6);
    assert_eq!(marked(&body), [ok]);

    // Without Discord set up nothing is marked, and the field isn't there at all.
    let plain = get(&state, None, uri).await;
    assert!(marked(&plain).is_empty());
    assert!(plain["clubs"][0].get("applications").is_none());

    for club in ids {
        store.unlink(club).await.unwrap();
    }
}

#[tokio::test]
async fn the_club_page_is_marked_the_same_way() {
    let Some(store) = store().await else {
        return;
    };
    let (open, closed) = (920_000_001, 920_000_002);
    let state = scanned(vec![detail(open, 2, 10), detail(closed, 3, 10)]).await;
    store.link(open, SERVER).await.unwrap();
    store.link(closed, SERVER).await.unwrap();

    let page = get(&state, Some(&store), &format!("/api/v1/clubs/{open}")).await;
    assert_eq!(page["club"]["applications"], "discord");
    let page = get(&state, Some(&store), &format!("/api/v1/clubs/{closed}")).await;
    assert!(page["club"].get("applications").is_none());

    let pause = SettingsUpdate {
        open: Some(false),
        ..SettingsUpdate::default()
    };
    store.update(open, SERVER, &pause).await.unwrap();
    let page = get(&state, Some(&store), &format!("/api/v1/clubs/{open}")).await;
    assert!(page["club"].get("applications").is_none());

    store.unlink(open).await.unwrap();
    store.unlink(closed).await.unwrap();
}

async fn redis_set(club: u64, field: &str, value: &str) {
    let url = std::env::var("REDIS_TEST_URL").unwrap();
    let mut conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .unwrap();
    redis::cmd("HSET")
        .arg(format!("discord:club:{club}"))
        .arg(field)
        .arg(value)
        .query_async::<()>(&mut conn)
        .await
        .unwrap();
}
