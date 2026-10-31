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
        "made-up-site-key",
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

async fn recommendations(app: Router, authorization: Option<&str>) -> StatusCode {
    let mut request = Request::get("/api/v1/recommendations?pace=500");
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    app.oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn recommendations_need_a_valid_token_when_the_check_is_on() {
    let server = cloudflare(true).await;
    let human = check(&server);
    let (_, body) = challenge(app(human.clone()), r#"{"response":"made-up-response"}"#).await;
    let token = body["token"].as_str().unwrap();

    for authorization in [
        None,
        Some(""),
        Some("Bearer "),
        Some("Bearer made-up-token"),
        Some(token),
    ] {
        assert_eq!(
            recommendations(app(human.clone()), authorization).await,
            StatusCode::UNAUTHORIZED,
            "{authorization:?}"
        );
    }
    // A valid token gets past the check; the first scan hasn't listed any clubs yet.
    assert_eq!(
        recommendations(app(human), Some(&format!("Bearer {token}"))).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn the_club_page_needs_a_valid_token_when_the_check_is_on() {
    let server = cloudflare(true).await;
    let human = check(&server);
    let (_, body) = challenge(app(human.clone()), r#"{"response":"made-up-response"}"#).await;
    let token = body["token"].as_str().unwrap();
    let club = |authorization: Option<String>| {
        let mut request = Request::get("/api/v1/clubs/1");
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        app(human.clone()).oneshot(request.body(Body::empty()).unwrap())
    };

    let status = club(None).await.unwrap().status();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A valid token gets past the check; no club is loaded yet.
    let status = club(Some(format!("Bearer {token}")))
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_from_another_key_is_turned_away() {
    let server = cloudflare(true).await;
    let (_, body) = challenge(app(check(&server)), r#"{"response":"made-up-response"}"#).await;
    let other = HumanCheck::new(
        "made-up-site-key",
        TurnstileSecret::parse("made-up-secret").unwrap(),
        SigningKey::parse(&"o".repeat(32)).unwrap(),
    );
    let token = format!("Bearer {}", body["token"].as_str().unwrap());
    assert_eq!(
        recommendations(app(other), Some(&token)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn health_stays_open_when_the_check_is_on() {
    let server = cloudflare(true).await;
    let health = app(check(&server))
        .oneshot(Request::get("/api/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
}

#[tokio::test]
async fn recommendations_need_no_token_when_the_check_is_off() {
    assert_eq!(
        recommendations(app(HumanCheck::default()), None).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn the_site_key_is_served_when_the_check_is_on_and_not_found_when_off() {
    let get = |human: HumanCheck| async move {
        let response = app(human)
            .oneshot(
                Request::get("/api/v1/challenge")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice::<Value>(&bytes).unwrap())
    };
    let server = cloudflare(true).await;
    let (status, body) = get(check(&server)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "site_key": "made-up-site-key" }));
    let (status, _) = get(HumanCheck::default()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
