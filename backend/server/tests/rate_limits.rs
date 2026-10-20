//! Rate limit tests. Addresses are from documentation ranges and all data is made up.

use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
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

fn details(client: &Client) -> ClubDetails {
    ClubDetails::new(client, SharedPool::default(), DetailConfig::default()).0
}

fn app(cache: Cache, details: ClubDetails, base_url: String, limits: RateLimits) -> Router {
    router(AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(client(base_url), DEFAULT_PACE_TTL),
        limits,
    })
}

fn offline(limits: RateLimits) -> Router {
    let base_url = "http://127.0.0.1:9".to_owned();
    let details = details(&client(base_url.clone()));
    app(Cache::new(), details, base_url, limits)
}

struct Call<'a> {
    path: &'a str,
    peer: Option<&'a str>,
    forwarded: Option<&'a str>,
}

impl<'a> Call<'a> {
    fn to(path: &'a str) -> Self {
        Self {
            path,
            peer: None,
            forwarded: None,
        }
    }

    fn from(mut self, peer: &'a str) -> Self {
        self.peer = Some(peer);
        self
    }

    fn forwarded(mut self, value: &'a str) -> Self {
        self.forwarded = Some(value);
        self
    }

    async fn send(self, app: &Router) -> (StatusCode, Option<String>, Value) {
        let mut request = Request::get(self.path);
        if let Some(value) = self.forwarded {
            request = request.header("x-forwarded-for", value);
        }
        let mut request = request.body(Body::empty()).unwrap();
        if let Some(peer) = self.peer {
            let addr: SocketAddr = peer.parse().unwrap();
            request.extensions_mut().insert(ConnectInfo(addr));
        }
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .map(|v| v.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            retry_after,
            serde_json::from_slice(&body).unwrap_or(Value::Null),
        )
    }
}

const RECOMMEND: &str = "/api/v1/recommendations?pace=7600000";

#[tokio::test]
async fn limits_requests_per_visitor() {
    let app = offline(RateLimits::new(2, 100, false));
    for _ in 0..2 {
        let (status, _, _) = Call::to(RECOMMEND)
            .from("203.0.113.1:5000")
            .send(&app)
            .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
    let (status, retry_after, body) = Call::to(RECOMMEND)
        .from("203.0.113.1:5001")
        .send(&app)
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let seconds: u64 = retry_after.unwrap().parse().unwrap();
    assert!((1..=30).contains(&seconds));
    assert_eq!(
        body["error"],
        format!("too many requests; try again in {seconds} seconds")
    );
}

#[tokio::test]
async fn visitors_have_separate_limits() {
    let app = offline(RateLimits::new(1, 100, false));
    let (first, _, _) = Call::to(RECOMMEND)
        .from("203.0.113.1:5000")
        .send(&app)
        .await;
    let (second, _, _) = Call::to(RECOMMEND)
        .from("203.0.113.2:5000")
        .send(&app)
        .await;
    let (again, _, _) = Call::to(RECOMMEND)
        .from("203.0.113.1:5000")
        .send(&app)
        .await;
    assert_eq!(first, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(again, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn ipv6_visitors_share_a_64_bit_prefix() {
    let app = offline(RateLimits::new(1, 100, false));
    let (first, _, _) = Call::to(RECOMMEND)
        .from("[2001:db8::1]:5000")
        .send(&app)
        .await;
    let (same_prefix, _, _) = Call::to(RECOMMEND)
        .from("[2001:db8::2]:5000")
        .send(&app)
        .await;
    let (other_prefix, _, _) = Call::to(RECOMMEND)
        .from("[2001:db8:0:1::1]:5000")
        .send(&app)
        .await;
    assert_eq!(first, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(same_prefix, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(other_prefix, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn forwarded_for_is_ignored_unless_trusted() {
    let app = offline(RateLimits::new(1, 100, false));
    let (first, _, _) = Call::to(RECOMMEND)
        .from("192.0.2.10:5000")
        .forwarded("203.0.113.1")
        .send(&app)
        .await;
    let (second, _, _) = Call::to(RECOMMEND)
        .from("192.0.2.10:5000")
        .forwarded("203.0.113.2")
        .send(&app)
        .await;
    assert_eq!(first, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn trusted_forwarded_for_uses_the_rightmost_address() {
    let app = offline(RateLimits::new(1, 100, true));
    let (first, _, _) = Call::to(RECOMMEND)
        .from("192.0.2.10:5000")
        .forwarded("198.51.100.1, 203.0.113.1")
        .send(&app)
        .await;
    let (other_visitor, _, _) = Call::to(RECOMMEND)
        .from("192.0.2.10:5000")
        .forwarded("203.0.113.2")
        .send(&app)
        .await;
    let (spoofed_left, _, _) = Call::to(RECOMMEND)
        .from("192.0.2.10:5000")
        .forwarded("198.51.100.99, 203.0.113.1")
        .send(&app)
        .await;
    assert_eq!(first, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(other_visitor, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(spoofed_left, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn health_and_status_are_not_limited() {
    let app = offline(RateLimits::new(1, 1, false));
    for path in ["/api/v1/health", "/api/v1/status"] {
        for _ in 0..3 {
            let (status, _, _) = Call::to(path).from("203.0.113.1:5000").send(&app).await;
            assert_eq!(status, StatusCode::OK, "{path}");
        }
    }
}

fn series(pace: i64) -> Vec<i64> {
    let mut fans = vec![0i64; 31];
    for (day, value) in fans.iter_mut().take(10).enumerate() {
        *value = 100_000_000 + pace * day as i64;
    }
    fans
}

async fn scanned_server() -> (MockServer, Cache, ClubDetails) {
    let server = MockServer::start().await;
    let members: Vec<Value> = (0..10)
        .map(|i| json!({ "viewer_id": 9000 + i, "year": 2026, "month": 3, "daily_fans": series(4_000_000 + 500_000 * i) }))
        .collect();
    let circle = json!({
        "circle_id": 1,
        "name": "Limit Club",
        "member_count": 10,
        "join_style": 1,
        "monthly_rank": 100,
        "monthly_point": 40_000_000,
        "last_updated": "2026-03-19T15:00:00Z",
    });
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "circles": [circle.clone()] })),
        )
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "circle": circle, "members": members })),
        )
        .mount(&server)
        .await;
    for trainer in [4001u64, 4002] {
        Mock::given(path("/api/v4/circles"))
            .and(query_param("viewer_id", trainer.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "circle": { "circle_id": 2 },
                "members": [{ "viewer_id": trainer, "year": 2026, "month": 3, "daily_fans": series(6_000_000) }],
            })))
            .mount(&server)
            .await;
    }

    let cache = Cache::new();
    let details = details(&client(server.uri()));
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client(server.uri()),
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
    (server, cache, details)
}

async fn lookups(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.query().is_some_and(|q| q.contains("viewer_id")))
        .count()
}

#[tokio::test]
async fn trainer_lookups_have_their_own_budget() {
    let (server, cache, details) = scanned_server().await;
    let app = app(cache, details, server.uri(), RateLimits::new(100, 1, false));
    let visitor = "203.0.113.1:5000";

    let (first, _, _) = Call::to("/api/v1/recommendations?trainer_id=4001")
        .from(visitor)
        .send(&app)
        .await;
    assert_eq!(first, StatusCode::OK);

    let (second, retry_after, _) = Call::to("/api/v1/recommendations?trainer_id=4002")
        .from(visitor)
        .send(&app)
        .await;
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
    assert!(retry_after.is_some());

    let (cached, _, _) = Call::to("/api/v1/recommendations?trainer_id=4001")
        .from(visitor)
        .send(&app)
        .await;
    assert_eq!(cached, StatusCode::OK);

    let (by_pace, _, _) = Call::to(RECOMMEND).from(visitor).send(&app).await;
    assert_eq!(by_pace, StatusCode::OK);

    assert_eq!(lookups(&server).await, 1);
}
