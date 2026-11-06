//! Webhook events endpoint tests with signed requests. All values are made up.
//!
//! The test that ends sessions needs Redis and runs only when `REDIS_TEST_URL` is set; CI sets it.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::json;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, KeySecret, PublicKey,
    PublicUrl, SessionStore, SigningKey as TokenKey, WEBHOOK_EVENTS_PATH,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

fn discord() -> Discord {
    let public = hex(signing_key().verifying_key().as_bytes());
    Discord::new(DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        PublicKey::parse(&public).unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        TokenKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    ))
    .unwrap()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn signed(body: &str, key: &SigningKey) -> Request<Body> {
    let timestamp = now().to_string();
    let signature = key.sign(format!("{timestamp}{body}").as_bytes());
    Request::post(WEBHOOK_EVENTS_PATH)
        .header("content-type", "application/json")
        .header("x-signature-ed25519", hex(&signature.to_bytes()))
        .header("x-signature-timestamp", timestamp)
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn deauthorized(user: &str) -> String {
    json!({
        "version": 1,
        "application_id": "1000",
        "type": 1,
        "event": {
            "type": "APPLICATION_DEAUTHORIZED",
            "timestamp": "2026-01-01T00:00:00.000000",
            "data": { "user": { "id": user, "username": "made-up-name" } },
        },
    })
    .to_string()
}

async fn send(app: axum::Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, body.to_vec())
}

#[tokio::test]
async fn answers_a_signed_ping_with_an_empty_204() {
    let ping = json!({ "version": 1, "application_id": "1000", "type": 0 }).to_string();
    let (status, body) = send(router(discord()), signed(&ping, &signing_key())).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

#[tokio::test]
async fn rejects_a_bad_or_missing_signature() {
    let other = SigningKey::from_bytes(&[8; 32]);
    let (status, _) = send(router(discord()), signed(&deauthorized("1"), &other)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let unsigned = Request::post(WEBHOOK_EVENTS_PATH)
        .body(Body::from(deauthorized("1")))
        .unwrap();
    let (status, _) = send(router(discord()), unsigned).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn other_events_are_acknowledged_and_ignored() {
    let event = json!({
        "version": 1,
        "application_id": "1000",
        "type": 1,
        "event": { "type": "APPLICATION_AUTHORIZED", "timestamp": "2026-01-01T00:00:00" },
    })
    .to_string();
    let (status, _) = send(router(discord()), signed(&event, &signing_key())).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn a_deauthorization_without_a_user_is_a_bad_request() {
    let event = json!({
        "version": 1,
        "application_id": "1000",
        "type": 1,
        "event": { "type": "APPLICATION_DEAUTHORIZED", "timestamp": "2026-01-01T00:00:00" },
    })
    .to_string();
    let (status, _) = send(router(discord()), signed(&event, &signing_key())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn removing_the_app_ends_every_session_of_that_person() {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    let secret = KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap();
    let sessions = SessionStore::new(conn, secret);
    let user = 100_000_000_000_000_000 + now() % 1_000_000_000;
    let (other, started) = (user + 1, now() - 60);
    assert!(sessions
        .valid("made-up.token", user, started)
        .await
        .unwrap());

    let app = router(discord().with_sessions(sessions.clone()));
    let (status, _) = send(
        app,
        signed(&deauthorized(&user.to_string()), &signing_key()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!sessions
        .valid("made-up.token", user, started)
        .await
        .unwrap());
    assert!(sessions
        .valid("made-up.other", other, started)
        .await
        .unwrap());
}
