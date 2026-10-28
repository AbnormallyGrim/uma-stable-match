//! Human check tests, with a stubbed Cloudflare. All keys and responses are made up.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_server::{
    router, AppState, Cache, ClubDetails, DetailConfig, HumanCheck, MatchSettings, PaceLookup,
    RateLimits, SharedPool, SigningKey, TurnstileSecret, DEFAULT_PACE_TTL, TOKEN_LIFETIME,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn app(human: HumanCheck) -> Router {
    let client = Client::new(ClientConfig::new(ApiKey::new("test-key"))).unwrap();
    router(AppState {
        cache: Cache::new(),
        details: ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
        human,
    })
}

fn check(server: &MockServer) -> HumanCheck {
    HumanCheck::new(
        TurnstileSecret::parse("made-up-secret").unwrap(),
        SigningKey::parse(&"k".repeat(32)).unwrap(),
    )
    .with_verify_url(format!("{}/siteverify", server.uri()))
}

async fn cloudflare(success: bool) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/siteverify"))
        .and(body_json(
            json!({ "secret": "made-up-secret", "response": "made-up-response" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "success": success })))
        .mount(&server)
        .await;
    server
}

async fn challenge(app: Router, body: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::post("/api/v1/challenge")
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn a_passed_challenge_gets_a_token_the_check_accepts() {
    let server = cloudflare(true).await;
    let human = check(&server);
    let (status, body) = challenge(app(human.clone()), r#"{"response":"made-up-response"}"#).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["expires_in"], TOKEN_LIFETIME.as_secs());
    assert!(human.is_valid(body["token"].as_str().unwrap()));
    assert!(!human.is_valid("made-up-token"));
}

#[tokio::test]
async fn a_failed_challenge_is_forbidden() {
    let server = cloudflare(false).await;
    let (status, body) = challenge(app(check(&server)), r#"{"response":"made-up-response"}"#).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.get("token").is_none());
}

#[tokio::test]
async fn an_unreachable_cloudflare_is_a_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let (status, _) = challenge(app(check(&server)), r#"{"response":"made-up-response"}"#).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn a_slow_cloudflare_is_a_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&server)
        .await;
    let (status, _) = challenge(app(check(&server)), r#"{"response":"made-up-response"}"#).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn empty_and_oversized_responses_are_rejected_without_calling_cloudflare() {
    let server = MockServer::start().await;
    for body in [
        json!({ "response": "" }).to_string(),
        json!({ "response": "x".repeat(3000) }).to_string(),
    ] {
        let (status, _) = challenge(app(check(&server)), &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_challenge_is_not_found_when_the_check_is_off() {
    let (status, _) = challenge(
        app(HumanCheck::default()),
        r#"{"response":"made-up-response"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
