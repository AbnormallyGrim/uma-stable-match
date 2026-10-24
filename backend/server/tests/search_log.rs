//! The per-search log line. Kept in its own test binary: tracing's scoped log capture can miss
//! events when other tests in the same binary log at the same time. All data is made up.

use std::sync::{Arc, Mutex};
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

fn client(base_url: String) -> Client {
    Client::new(ClientConfig {
        base_url,
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn detail(id: u64, pace: i64) -> Value {
    let members: Vec<Value> = (0..10)
        .map(|i| {
            let mut fans = vec![0i64; 31];
            for (day, value) in fans.iter_mut().take(10).enumerate() {
                *value = 100_000_000 + (pace + 100_000 * i) * day as i64;
            }
            json!({ "viewer_id": 8000 + id * 100 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    json!({
        "circle": {
            "circle_id": id,
            "name": format!("Log Club {id}"),
            "member_count": 10,
            "join_style": 1,
            "monthly_rank": 100 + id,
            "monthly_point": 20_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        },
        "members": members,
    })
}

async fn scanned() -> AppState {
    let server = MockServer::start().await;
    let clubs = [detail(1, 4_500_000), detail(2, 5_500_000)];
    let listed: Vec<Value> = clubs.iter().map(|d| d["circle"].clone()).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": listed })))
        .mount(&server)
        .await;
    for club in clubs {
        Mock::given(path("/api/v4/circles"))
            .and(query_param(
                "circle_id",
                club["circle"]["circle_id"].to_string(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(club))
            .mount(&server)
            .await;
    }

    let client = client(server.uri());
    let cache = Cache::new();
    let details = ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0;
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client.clone(),
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
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
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

#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Logs {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Logs {
    fn lines(&self, message: &str) -> Vec<Value> {
        String::from_utf8(self.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|line| line["message"] == message)
            .collect()
    }
}

#[tokio::test]
async fn searches_are_logged_so_they_can_be_rerun() {
    let state = scanned().await;
    let logs = Logs::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_writer(logs.clone())
            .finish(),
    );
    let (status, body) = get(&state, "?pace=5000000&limit=3").await;
    let (rejected, _) = get(&state, "?pace=fast").await;
    drop(guard);

    assert_eq!(status, StatusCode::OK);
    let searches = logs.lines("search");
    assert_eq!(searches.len(), 1);
    let search = &searches[0];
    assert_eq!(search["level"], "INFO");
    assert_eq!(search["query"], "pace=5000000&limit=3");
    assert_eq!(search["status"], 200);
    assert_eq!(search["pace_per_day"], 5_000_000.0);
    assert_eq!(search["total"], body["total"]);
    let top: Vec<u64> = body["clubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|club| club["circle_id"].as_u64().unwrap())
        .collect();
    assert_eq!(search["top_clubs"], format!("{top:?}"));
    assert!(search["duration_ms"].is_u64());
    assert_eq!(search["shortlist_rank"], 102);
    assert_eq!(search["pick_rank"], 102);
    assert_eq!(search["pool_bottom"], true);

    assert_eq!(rejected, StatusCode::BAD_REQUEST);
    let failed = logs.lines("search failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["query"], "pace=fast");
    assert_eq!(failed[0]["status"], 400);
    assert!(failed[0]["error"].as_str().unwrap().contains("pace"));
}
