//! REST client tests against a mock Discord.

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use uma_stable_match_discord::{
    ApplicationId, BotToken, ClientSecret, DiscordConfig, Error, PublicKey, RateLimitConfig, Rest,
    StatusCode,
};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config(server: &MockServer) -> DiscordConfig {
    let mut config = DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        // The Ed25519 base point: a valid, made-up public key.
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
    );
    config.api_base = server.uri();
    config.rate_limit = RateLimitConfig {
        max_wait: Duration::from_secs(2),
        max_retries: 2,
    };
    config
}

#[tokio::test]
async fn sends_the_bot_token_and_reads_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/guilds/111"))
        .and(header("authorization", "Bot made-up-bot-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "111" })))
        .expect(1)
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let guild: Value = rest.get("/guilds/111").await.unwrap();
    assert_eq!(guild["id"], "111");

    let request = &server.received_requests().await.unwrap()[0];
    let agent = request.headers.get("user-agent").unwrap().to_str().unwrap();
    assert!(agent.starts_with("DiscordBot ("), "{agent}");
}

#[tokio::test]
async fn sends_json_bodies_and_reads_empty_responses() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/guilds/111/members/222"))
        .and(body_json(json!({ "access_token": "made-up" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let () = rest
        .put(
            "/guilds/111/members/222",
            &json!({ "access_token": "made-up" }),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn reports_discord_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/guilds/111"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({ "code": 10004, "message": "Unknown Guild" })),
        )
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let error = rest.get::<Value>("/guilds/111").await.unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::NOT_FOUND));
    assert!(
        matches!(&error, Error::Api { code: Some(10004), message, .. } if message == "Unknown Guild")
    );
}

#[tokio::test]
async fn retries_after_a_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/guilds/111"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "message": "You are being rate limited.",
            "retry_after": 0.2,
            "global": false,
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/guilds/111"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "111" })))
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let start = Instant::now();
    let guild: Value = rest.get("/guilds/111").await.unwrap();
    assert_eq!(guild["id"], "111");
    assert!(start.elapsed() >= Duration::from_millis(200));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn gives_up_when_the_wait_is_too_long() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({ "retry_after": 60.0 })))
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let error = rest.get::<Value>("/guilds/111").await.unwrap_err();
    assert!(matches!(error, Error::RateLimited(wait) if wait == Duration::from_secs(60)));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // The route stays limited, so the next call fails without a request.
    rest.get::<Value>("/guilds/111").await.unwrap_err();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn gives_up_after_the_last_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({ "retry_after": 0.01 })))
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    let error = rest.get::<Value>("/guilds/111").await.unwrap_err();
    assert!(matches!(error, Error::RateLimited(_)));
    // The first try and 2 retries.
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn waits_for_an_empty_bucket_to_reset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("x-ratelimit-reset-after", "0.3")
                .set_body_json(json!({})),
        )
        .mount(&server)
        .await;
    let rest = Rest::new(&config(&server)).unwrap();
    rest.get::<Value>("/guilds/111").await.unwrap();
    let start = Instant::now();
    rest.get::<Value>("/guilds/111").await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(250));

    // Another guild is another bucket, so it doesn't wait.
    let start = Instant::now();
    rest.get::<Value>("/guilds/222").await.unwrap();
    assert!(start.elapsed() < Duration::from_millis(250));
}
