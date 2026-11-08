//! Discord interactions endpoint mounting tests. All values are made up.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router as discord_router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig,
    PublicKey, PublicUrl, SigningKey, INTERACTIONS_PATH,
};
use uma_stable_match_server::{
    app, app_with_routes, AppState, Cache, ClubDetails, DetailConfig, Features, MatchSettings,
    PaceLookup, RateLimits, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};

fn state() -> AppState {
    let client = Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    AppState {
        cache: Cache::new(),
        details: ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

fn discord() -> Discord {
    Discord::new(DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        // The Ed25519 base point: a valid public key.
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        SigningKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    ))
    .unwrap()
}

fn unsigned_ping() -> Request<Body> {
    Request::post(INTERACTIONS_PATH)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"type":1}"#))
        .unwrap()
}

#[tokio::test]
async fn interactions_are_served_with_the_security_headers() {
    let app = app_with_routes(
        state(),
        None,
        Features::default(),
        discord_router(discord()),
    );
    let response = app.oneshot(unsigned_ping()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn interactions_arent_served_without_discord() {
    let response = app(state(), None).oneshot(unsigned_ping()).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sign_in_routes_are_served_before_the_api_not_found() {
    let app = app_with_routes(
        state(),
        None,
        Features::default(),
        discord_router(discord()),
    );
    let response = app
        .oneshot(
            Request::get("/api/v1/auth/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], br#"{"error":"not signed in"}"#);
}
