//! Recommendations route tests against a mock server. All data is made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use uma_stable_match_scoring::{score_clubs, Club, ScoringOptions};
use uma_stable_match_server::{
    router, run_scheduler, AppState, Cache, PaceLookup, ScanConfig, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, CircleResponse, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const M: f64 = 1_000_000.0;

fn app(cache: Cache) -> AppState {
    let client = Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    AppState {
        cache,
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
    }
}

fn series(pace: f64) -> Vec<i64> {
    let mut fans = vec![0i64; 31];
    for (day, value) in fans.iter_mut().take(10).enumerate() {
        *value = 100_000_000 + (pace * day as f64) as i64;
    }
    fans
}

fn detail(id: u64, join_style: i32, member_count: i32, paces: &[f64]) -> Value {
    let members: Vec<Value> = paces
        .iter()
        .enumerate()
        .map(|(i, pace)| {
            json!({
                "viewer_id": 8000 + id * 100 + i as u64,
                "year": 2026,
                "month": 3,
                "daily_fans": series(*pace),
            })
        })
        .collect();
    json!({
        "circle": {
            "circle_id": id,
            "name": format!("Advice Club {id}"),
            "member_count": member_count,
            "join_style": join_style,
            "policy": 2,
            "monthly_rank": 100 + id,
            "last_month_rank": 110 + id,
            "comment": "Made-up club",
        },
        "members": members,
        "club_rank": 7,
    })
}

fn paces(from: f64, step: f64, n: usize) -> Vec<f64> {
    (0..n).map(|i| from + step * i as f64).collect()
}

fn details() -> Vec<Value> {
    vec![
        detail(1, 1, 12, &paces(4.0 * M, 1.0 * M, 12)),
        detail(2, 2, 25, &paces(2.0 * M, 0.5 * M, 10)),
        detail(3, 3, 10, &paces(6.0 * M, 0.5 * M, 10)),
        detail(4, 1, 5, &paces(5.0 * M, 1.0 * M, 5)),
    ]
}

fn clubs() -> Vec<Club> {
    details()
        .into_iter()
        .map(|d| Club::from(serde_json::from_value::<CircleResponse>(d).unwrap()))
        .collect()
}

async fn scanned_cache() -> Cache {
    let server = MockServer::start().await;
    let listed: Vec<Value> = details().iter().map(|d| d["circle"].clone()).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": listed })))
        .mount(&server)
        .await;
    for d in details() {
        let id = d["circle"]["circle_id"].as_u64().unwrap();
        Mock::given(path("/api/v4/circles"))
            .and(query_param("circle_id", id.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(d))
            .mount(&server)
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
    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client,
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
    cache
}

async fn get(cache: &Cache, query: &str) -> (StatusCode, Value) {
    let response = router(app(cache.clone()))
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

fn assert_close(actual: &Value, expected: f64) {
    let actual = actual.as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "got {actual}, expected {expected}"
    );
}

fn ids(body: &Value) -> Vec<u64> {
    body["clubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["circle_id"].as_u64().unwrap())
        .collect()
}

#[tokio::test]
async fn unavailable_before_first_scan() {
    let (status, body) = get(&Cache::new(), "?pace=7600000").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "no scan results yet; try again shortly");
}

#[tokio::test]
async fn rejects_invalid_input() {
    let cache = Cache::new();
    let cases = [
        ("", "pace or trainer_id is required"),
        ("?pace=", "pace or trainer_id is required"),
        ("?pace=fast", "pace must be a number"),
        ("?pace=inf", "pace must be a number"),
        ("?pace=0", "pace must be greater than zero, got 0"),
        ("?pace=-5", "pace must be greater than zero, got -5"),
        (
            "?pace=5000000&balance_weight=x",
            "balance_weight must be a number",
        ),
        (
            "?pace=5000000&balance_weight=1.5",
            "balance weight must be between 0 and 1, got 1.5",
        ),
        (
            "?pace=5000000&include_closed=yes",
            "include_closed must be true or false",
        ),
        ("?pace=5000000&balance=yes", "balance must be true or false"),
        (
            "?pace=5000000&balance=false&balance_weight=0.5",
            "balance_weight can't be used with balance=false",
        ),
        (
            "?pace=5000000&limit=0",
            "limit must be a positive whole number",
        ),
        (
            "?pace=5000000&limit=-1",
            "limit must be a positive whole number",
        ),
        (
            "?pace=5000000&limit=many",
            "limit must be a positive whole number",
        ),
    ];
    let scanned = scanned_cache().await;
    for (query, message) in cases {
        let target = if message.starts_with("pace must be greater")
            || message.starts_with("balance weight must be between")
        {
            &scanned
        } else {
            &cache
        };
        let (status, body) = get(target, query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(body["error"], message, "{query}");
    }
}

#[tokio::test]
async fn ranks_clubs_like_the_scoring_crate() {
    let cache = scanned_cache().await;
    let pace = 7.6 * M;
    let (status, body) = get(&cache, "?pace=7600000").await;
    assert_eq!(status, StatusCode::OK);

    let clubs = clubs();
    let expected = score_clubs(pace, &clubs, &ScoringOptions::default()).unwrap();
    let expected_ids: Vec<u64> = expected.iter().map(|s| s.club.circle_id).collect();
    assert_eq!(ids(&body), expected_ids);
    assert!(!expected_ids.contains(&3), "closed club excluded");
    assert!(!expected_ids.contains(&4), "too few members");

    for (result, want) in body["clubs"].as_array().unwrap().iter().zip(&expected) {
        assert_close(&result["fit"], want.fit);
        assert_close(&result["score"], want.score);
        assert_eq!(result["your_place"], want.your_place);
        assert_eq!(result["club_size"], want.club_size);
        assert_close(&result["median_per_day"], want.median_per_day);
        assert_eq!(result["balance"], Value::Null);
    }

    assert_close(&body["trainer"]["pace_per_day"], pace);
    assert_eq!(body["trainer"]["trainer_id"], Value::Null);
    assert_eq!(body["options"]["balance_weight"], Value::Null);
    assert_eq!(body["options"]["include_closed"], false);
    assert_eq!(body["options"]["limit"], 50);
    assert_eq!(body["total"], expected.len());
    assert!(body["updated_at"].as_str().unwrap().ends_with('Z'));
}

#[tokio::test]
async fn returns_club_details() {
    let cache = scanned_cache().await;
    let (_, body) = get(&cache, "?pace=7600000").await;
    let club = body["clubs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circle_id"] == 2)
        .unwrap();
    assert_eq!(club["name"], "Advice Club 2");
    assert_eq!(club["url"], "https://uma.moe/circles/2");
    assert_eq!(club["tier"], "A");
    assert_eq!(club["join_style"], "approval");
    assert_eq!(club["policy"], 2);
    assert_eq!(club["member_count"], 25);
    assert_eq!(club["open_spots"], 5);
    assert_eq!(club["monthly_rank"], 102);
    assert_eq!(club["last_month_rank"], 112);
    assert_eq!(club["comment"], "Made-up club");
    assert_eq!(club["ranked_members"], 10);
}

#[tokio::test]
async fn includes_closed_clubs_on_request() {
    let cache = scanned_cache().await;
    let (_, body) = get(&cache, "?pace=7600000&include_closed=true").await;
    assert!(ids(&body).contains(&3));
    assert_eq!(body["options"]["include_closed"], true);
}

#[tokio::test]
async fn adds_balance_when_weighted() {
    let cache = scanned_cache().await;
    let (status, body) = get(&cache, "?pace=7600000&balance_weight=0.5").await;
    assert_eq!(status, StatusCode::OK);

    let clubs = clubs();
    let options = ScoringOptions {
        balance_weight: Some(0.5),
        include_closed: false,
    };
    let expected = score_clubs(7.6 * M, &clubs, &options).unwrap();
    assert_eq!(
        ids(&body),
        expected
            .iter()
            .map(|s| s.club.circle_id)
            .collect::<Vec<_>>()
    );
    for (result, want) in body["clubs"].as_array().unwrap().iter().zip(&expected) {
        let balance = want.balance.unwrap();
        assert_close(&result["score"], want.score);
        assert_eq!(result["balance"]["score"], balance.score);
        assert_eq!(result["balance"]["passengers"], balance.passengers);
        assert_eq!(result["balance"]["counted"], balance.counted);
        assert_close(&result["balance"]["gini"], balance.gini);
    }
    assert_eq!(body["options"]["balance_weight"].as_f64().unwrap(), 0.5);
}

#[tokio::test]
async fn balance_true_uses_the_default_weight() {
    let cache = scanned_cache().await;
    let (status, body) = get(&cache, "?pace=7600000&balance=true").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["options"]["balance_weight"].as_f64().unwrap(), 0.5);
    assert!(body["clubs"][0]["balance"].is_object());

    let (_, explicit) = get(&cache, "?pace=7600000&balance=true&balance_weight=1").await;
    assert_eq!(explicit["options"]["balance_weight"].as_f64().unwrap(), 1.0);

    let (_, off) = get(&cache, "?pace=7600000&balance=false").await;
    assert_eq!(off["options"]["balance_weight"], Value::Null);
    assert_eq!(off["clubs"][0]["balance"], Value::Null);
}

#[tokio::test]
async fn limit_caps_clubs_but_not_total() {
    let cache = scanned_cache().await;
    let (_, all) = get(&cache, "?pace=7600000&include_closed=true").await;
    let total = all["total"].as_u64().unwrap();
    assert!(total >= 2);

    let (status, body) = get(&cache, "?pace=7600000&include_closed=true&limit=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["clubs"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"].as_u64().unwrap(), total);
    assert_eq!(body["options"]["limit"], 1);
    assert_eq!(body["clubs"][0], all["clubs"][0]);
}
