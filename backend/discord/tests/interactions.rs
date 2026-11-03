//! Interactions endpoint tests with signed requests.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, PublicKey, PublicUrl,
    SigningKey as TokenKey, INTERACTIONS_PATH,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

fn app() -> axum::Router {
    let public = hex(signing_key().verifying_key().as_bytes());
    let config = DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        PublicKey::parse(&public).unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        TokenKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    );
    router(Discord::new(config).unwrap())
}

fn now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

fn signed(body: &str, key: &SigningKey, timestamp: &str) -> Request<Body> {
    let signature = key.sign(format!("{timestamp}{body}").as_bytes());
    Request::post(INTERACTIONS_PATH)
        .header("content-type", "application/json")
        .header("x-signature-ed25519", hex(&signature.to_bytes()))
        .header("x-signature-timestamp", timestamp)
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn send(request: Request<Body>) -> (StatusCode, Value) {
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn answers_a_signed_ping() {
    let (status, body) = send(signed(r#"{"type":1}"#, &signing_key(), &now())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "type": 1 }));
}

#[tokio::test]
async fn rejects_a_bad_signature() {
    let other = SigningKey::from_bytes(&[8; 32]);
    let (status, body) = send(signed(r#"{"type":1}"#, &other, &now())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "invalid request signature" }));
}

#[tokio::test]
async fn rejects_a_missing_signature() {
    let request = Request::post(INTERACTIONS_PATH)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"type":1}"#))
        .unwrap();
    let (status, _) = send(request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_a_replayed_request() {
    let (status, _) = send(signed(r#"{"type":1}"#, &signing_key(), "1700000000")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_a_signed_body_that_isnt_an_interaction() {
    let (status, _) = send(signed("[]", &signing_key(), &now())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn answers_other_interactions_privately() {
    let (status, body) = send(signed(r#"{"type":3}"#, &signing_key(), &now())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["type"], 4);
    assert_eq!(body["data"]["flags"], 64);
}
