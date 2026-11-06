//! Per-visitor limits on the Discord routes. All values are made up.

use axum::body::Body;
use axum::http::header::{ACCEPT, LOCATION, RETRY_AFTER};
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router as discord_router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig,
    PublicKey, PublicUrl, SigningKey, APPLICATIONS_ME_PATH, APPLY_PATH, CALLBACK_PATH,
    INTERACTIONS_PATH, LOGIN_PATH, OPT_IN_PATH, WEBHOOK_EVENTS_PATH,
};
use uma_stable_match_server::{limit_discord, RateLimits};

/// The Discord routes with 2 requests a minute and 1 trip to Discord an hour per visitor.
fn app() -> Router {
    let discord = Discord::new(DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        // The Ed25519 base point: a valid public key.
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        SigningKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    ))
    .unwrap();
    let limits = RateLimits::new(10_000, 10_000, true).with_discord(2, 1);
    limit_discord(discord_router(discord), limits)
}

async fn status(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone().oneshot(request).await.unwrap().status()
}

fn get(uri: &str, visitor: &str) -> Request<Body> {
    Request::get(uri)
        .header("x-forwarded-for", visitor)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn the_discord_routes_allow_a_few_requests_a_minute() {
    let app = app();
    for _ in 0..2 {
        assert_eq!(
            status(&app, get(APPLICATIONS_ME_PATH, "192.0.2.1")).await,
            StatusCode::UNAUTHORIZED
        );
    }
    let response = app
        .clone()
        .oneshot(get(LOGIN_PATH, "192.0.2.1"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(RETRY_AFTER));
    // Another visitor has their own count.
    assert_eq!(
        status(&app, get(APPLICATIONS_ME_PATH, "192.0.2.2")).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn trips_to_discord_share_a_stricter_limit_of_their_own() {
    for path in [CALLBACK_PATH, APPLY_PATH, OPT_IN_PATH] {
        let app = app();
        assert_ne!(
            status(&app, get(path, "192.0.2.1")).await,
            StatusCode::TOO_MANY_REQUESTS,
            "{path}"
        );
        for other in [CALLBACK_PATH, APPLY_PATH, OPT_IN_PATH] {
            assert_eq!(
                status(&app, get(other, "192.0.2.1")).await,
                StatusCode::TOO_MANY_REQUESTS,
                "{path} then {other}"
            );
        }
        // The per-minute routes keep their own count.
        assert_eq!(
            status(&app, get(APPLICATIONS_ME_PATH, "192.0.2.1")).await,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
}

#[tokio::test]
async fn a_limited_page_load_lands_on_the_applications_page() {
    let app = app();
    status(&app, get(CALLBACK_PATH, "192.0.2.1")).await;
    let mut request = get(CALLBACK_PATH, "192.0.2.1");
    request
        .headers_mut()
        .insert(ACCEPT, "text/html,*/*".parse().unwrap());
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers()[LOCATION],
        "/applications?error=rate_limited"
    );
}

#[tokio::test]
async fn interactions_and_webhook_events_are_not_limited() {
    let app = app();
    for path in [INTERACTIONS_PATH, WEBHOOK_EVENTS_PATH]
        .into_iter()
        .cycle()
        .take(10)
    {
        let request = Request::post(path)
            .header("x-forwarded-for", "192.0.2.1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"type":1}"#))
            .unwrap();
        // Unsigned, so refused, but never rate limited.
        assert_eq!(status(&app, request).await, StatusCode::UNAUTHORIZED);
    }
}
