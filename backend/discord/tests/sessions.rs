//! Ending sessions early. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use redis::aio::ConnectionManager;
use serde_json::json;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, KeySecret, PublicKey,
    PublicUrl, SessionStore, SigningKey, ACCOUNT_PATH, CALLBACK_PATH, LOGIN_PATH, MAX_SESSION_AGE,
    SESSION_PATH, SIGN_OUT_PATH,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A store, or `None` without `REDIS_TEST_URL`.
async fn connect() -> Option<SessionStore> {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    let secret = KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap();
    Some(SessionStore::new(conn, secret))
}

/// A Discord ID no other test, or run of the tests, uses.
fn user_id() -> u64 {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let run = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        % 1_000_000_000;
    100_000_000_000_000_000 + run * 1_000 + CALLS.fetch_add(1, Ordering::Relaxed)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn app(server: &MockServer, sessions: SessionStore, user: u64) -> Router {
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "access_token": "made-up-access-token", "token_type": "Bearer" }),
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/@me"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "id": user.to_string(), "username": "made-up-name" })),
        )
        .mount(server)
        .await;
    let mut config = DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        // The Ed25519 base point: a valid, made-up public key.
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        SigningKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    );
    config.api_base = server.uri();
    router(Discord::new(config).unwrap().with_sessions(sessions))
}

async fn send(app: &Router, method: &str, uri: &str, cookie: Option<&str>) -> Response<Body> {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(COOKIE, cookie);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// `name=value` of the cookie the response sets under `name`.
fn cookie(response: &Response<Body>, name: &str) -> String {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|c| c.starts_with(&format!("{name}=")))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

/// A session cookie, from signing in.
async fn signed_in(app: &Router) -> String {
    let login = send(app, "GET", &format!("{LOGIN_PATH}?next=/"), None).await;
    let location = login.headers()[LOCATION].to_str().unwrap();
    let url = reqwest::Url::parse(location).unwrap();
    let (_, state) = url.query_pairs().find(|(k, _)| k == "state").unwrap();
    let nonce = cookie(&login, "uma_discord_nonce");
    let callback = format!("{CALLBACK_PATH}?code=made-up-code&state={state}");
    let response = send(app, "GET", &callback, Some(&nonce)).await;
    cookie(&response, "uma_session")
}

#[tokio::test]
async fn a_signed_out_session_stops_working_even_as_a_copy() {
    let Some(sessions) = connect().await else {
        return;
    };
    let server = MockServer::start().await;
    let app = app(&server, sessions, user_id()).await;
    let session = signed_in(&app).await;
    let me = send(&app, "GET", SESSION_PATH, Some(&session)).await;
    assert_eq!(me.status(), StatusCode::OK);

    let out = send(&app, "POST", SIGN_OUT_PATH, Some(&session)).await;
    assert_eq!(out.status(), StatusCode::NO_CONTENT);
    let me = send(&app, "GET", SESSION_PATH, Some(&session)).await;
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deleting_the_account_ends_its_session() {
    let Some(sessions) = connect().await else {
        return;
    };
    let server = MockServer::start().await;
    let app = app(&server, sessions, user_id()).await;
    let session = signed_in(&app).await;
    let deleted = send(&app, "DELETE", ACCOUNT_PATH, Some(&session)).await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let me = send(&app, "GET", SESSION_PATH, Some(&session)).await;
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ending_every_session_ends_only_the_ones_started_before() {
    let Some(sessions) = connect().await else {
        return;
    };
    let (user, other) = (user_id(), user_id());
    let now = now();
    sessions.end_all(user, now).await.unwrap();
    assert!(!sessions.valid("made-up.old", user, now - 1).await.unwrap());
    // Signing in again straight away works.
    assert!(sessions.valid("made-up.new", user, now).await.unwrap());
    assert!(sessions
        .valid("made-up.other", other, now - 1)
        .await
        .unwrap());
}

#[tokio::test]
async fn a_revoked_token_is_kept_until_it_would_have_expired() {
    let Some(sessions) = connect().await else {
        return;
    };
    let user = user_id();
    let token = format!("made-up-token-{user}");
    let now = now();
    let started = now - 60;
    let expires = started + MAX_SESSION_AGE.as_secs();
    sessions.revoke(&token, expires, now).await.unwrap();
    assert!(!sessions.valid(&token, user, started).await.unwrap());
    assert!(sessions
        .valid(&format!("{token}-other"), user, started)
        .await
        .unwrap());
}
