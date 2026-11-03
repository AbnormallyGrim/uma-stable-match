//! Sign-in tests against a mock Discord. All values are made up.

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, PublicKey, PublicUrl,
    SigningKey, CALLBACK_PATH, LEADERS_ME_PATH, LOGIN_PATH, OPT_IN_PATH, SIGN_OUT_PATH,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const USER_ID: &str = "123456789012345678";

fn app(server: &MockServer) -> Router {
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
    router(Discord::new(config).unwrap())
}

async fn get(app: &Router, uri: &str, cookie: Option<&str>) -> Response<Body> {
    let mut request = Request::get(uri);
    if let Some(cookie) = cookie {
        request = request.header(COOKIE, cookie);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

fn header_value(response: &Response<Body>, name: impl axum::http::header::AsHeaderName) -> String {
    response.headers()[name].to_str().unwrap().to_owned()
}

fn set_cookies(response: &Response<Body>) -> Vec<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect()
}

/// `name=value` of the cookie set under `name`.
fn cookie(response: &Response<Body>, name: &str) -> String {
    let set = set_cookies(response)
        .into_iter()
        .find(|c| c.starts_with(&format!("{name}=")))
        .unwrap();
    set.split(';').next().unwrap().to_owned()
}

async fn json_body(response: Response<Body>) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

fn query(location: &str, name: &str) -> String {
    let url = reqwest::Url::parse(location).unwrap();
    let (_, value) = url.query_pairs().find(|(k, _)| k == name).unwrap();
    value.into_owned()
}

/// Starts a sign-in, returning the `state` Discord would send back and the nonce cookie.
async fn start(app: &Router, next: &str) -> (String, String) {
    let response = get(app, &format!("{LOGIN_PATH}?next={next}"), None).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let state = query(&header_value(&response, LOCATION), "state");
    (state, cookie(&response, "uma_discord_nonce"))
}

async fn mock_discord(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .and(body_string_contains("code=made-up-code"))
        .and(body_string_contains("client_id=1000"))
        .and(body_string_contains("client_secret=made-up-client-secret"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains(
            "redirect_uri=https%3A%2F%2Fexample.test%2Fapi%2Fv1%2Fauth%2Fdiscord%2Fcallback",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "access_token": "made-up-access-token", "token_type": "Bearer" }),
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/@me"))
        .and(header("authorization", "Bearer made-up-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "id": USER_ID, "username": "made-up-name", "global_name": "Made Up" }),
        ))
        .mount(server)
        .await;
}

fn callback_uri(state: &str, code: &str) -> String {
    format!("{CALLBACK_PATH}?code={code}&state={state}")
}

#[tokio::test]
async fn login_redirects_to_discord_with_a_signed_state_and_a_nonce_cookie() {
    let server = MockServer::start().await;
    let app = app(&server);
    let response = get(&app, LOGIN_PATH, None).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = header_value(&response, LOCATION);
    assert!(location.starts_with("https://discord.com/oauth2/authorize?"));
    assert_eq!(query(&location, "client_id"), "1000");
    assert_eq!(query(&location, "response_type"), "code");
    assert_eq!(query(&location, "scope"), "identify");
    assert_eq!(
        query(&location, "redirect_uri"),
        "https://example.test/api/v1/auth/discord/callback"
    );
    let nonce = set_cookies(&response).remove(0);
    assert!(nonce.starts_with("uma_discord_nonce="));
    for attribute in ["HttpOnly", "Secure", "SameSite=Lax", "Max-Age=600"] {
        assert!(nonce.contains(attribute), "{nonce}");
    }
    assert!(!nonce.contains(&query(&location, "state")));
}

#[tokio::test]
async fn flows_other_than_the_leaders_arent_available_yet() {
    let server = MockServer::start().await;
    let app = app(&server);
    for flow in ["opt_in", "apply", "nope"] {
        let response = get(&app, &format!("{LOGIN_PATH}?flow={flow}"), None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{flow}");
    }
}

#[tokio::test]
async fn signing_in_starts_a_leader_session_and_returns_to_the_page() {
    let server = MockServer::start().await;
    mock_discord(&server).await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/club/1").await;

    let response = get(&app, &callback_uri(&state, "made-up-code"), Some(&nonce)).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(header_value(&response, LOCATION), "/club/1");
    let cookies = set_cookies(&response);
    let session = cookies
        .iter()
        .find(|c| c.starts_with("uma_leader_session="))
        .unwrap();
    for attribute in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Max-Age=43200",
        "Path=/;",
    ] {
        assert!(session.contains(attribute), "{session}");
    }
    assert!(
        !session.contains(USER_ID),
        "the ID is only inside the signed value"
    );
    let cleared = cookies
        .iter()
        .find(|c| c.starts_with("uma_discord_nonce="))
        .unwrap();
    assert!(cleared.contains("Max-Age=0"));

    let me = get(
        &app,
        LEADERS_ME_PATH,
        Some(&cookie(&response, "uma_leader_session")),
    )
    .await;
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(header_value(&me, CACHE_CONTROL), "no-store");
    assert_eq!(json_body(me).await, json!({ "discord_id": USER_ID }));
}

#[tokio::test]
async fn the_session_cookie_holds_nothing_from_discord_but_the_signed_id() {
    let server = MockServer::start().await;
    mock_discord(&server).await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;
    let response = get(&app, &callback_uri(&state, "made-up-code"), Some(&nonce)).await;
    let session = cookie(&response, "uma_leader_session");
    for secret in ["made-up-access-token", "made-up-name", "Made Up"] {
        assert!(!session.contains(secret));
    }
}

#[tokio::test]
async fn a_callback_without_the_nonce_cookie_is_rejected() {
    let server = MockServer::start().await;
    mock_discord(&server).await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;

    // Forged: opened in another browser, which has no cookie.
    let response = get(&app, &callback_uri(&state, "made-up-code"), None).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(set_cookies(&response)
        .iter()
        .all(|c| !c.starts_with("uma_leader_session=")));
    // Mixed up with another sign-in's nonce.
    let (_, other) = start(&app, "/").await;
    assert_ne!(nonce, other);
    let response = get(&app, &callback_uri(&state, "made-up-code"), Some(&other)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_replayed_callback_is_rejected_once_the_nonce_is_cleared() {
    let server = MockServer::start().await;
    mock_discord(&server).await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;
    let first = get(&app, &callback_uri(&state, "made-up-code"), Some(&nonce)).await;
    assert_eq!(first.status(), StatusCode::TEMPORARY_REDIRECT);
    // The browser dropped the cleared cookie, so a replay carries none.
    let replay = get(&app, &callback_uri(&state, "made-up-code"), None).await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_tampered_state_is_rejected_without_calling_discord() {
    let server = MockServer::start().await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;
    let tampered = format!("{state}x");
    for state in [tampered.as_str(), "garbage", ""] {
        let response = get(&app, &callback_uri(state, "made-up-code"), Some(&nonce)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{state:?}");
    }
    let response = get(
        &app,
        &format!("{CALLBACK_PATH}?code=made-up-code"),
        Some(&nonce),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_on_discord_is_rejected() {
    let server = MockServer::start().await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;
    let uri = format!("{CALLBACK_PATH}?error=access_denied&state={state}");
    let response = get(&app, &uri, Some(&nonce)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_code_discord_refuses_is_a_bad_gateway_with_no_session() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(
            json!({ "error": "invalid_grant", "message": "Invalid \"code\" in request." }),
        ))
        .mount(&server)
        .await;
    let app = app(&server);
    let (state, nonce) = start(&app, "/").await;
    let response = get(&app, &callback_uri(&state, "made-up-code"), Some(&nonce)).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(set_cookies(&response)
        .iter()
        .all(|c| !c.starts_with("uma_leader_session=")));
}

#[tokio::test]
async fn next_outside_the_site_is_ignored() {
    let server = MockServer::start().await;
    mock_discord(&server).await;
    let app = app(&server);
    let (state, nonce) = start(&app, "//evil.test").await;
    let response = get(&app, &callback_uri(&state, "made-up-code"), Some(&nonce)).await;
    assert_eq!(header_value(&response, LOCATION), "/");
}

#[tokio::test]
async fn leaders_me_needs_a_valid_session() {
    let server = MockServer::start().await;
    let app = app(&server);
    let response = get(&app, LEADERS_ME_PATH, None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(header_value(&response, CACHE_CONTROL), "no-store");
    for forged in [
        "uma_leader_session=abc.def",
        "uma_leader_session=",
        "other=1",
    ] {
        let response = get(&app, LEADERS_ME_PATH, Some(forged)).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{forged}");
    }
}

#[tokio::test]
async fn a_state_is_not_a_session() {
    let server = MockServer::start().await;
    let app = app(&server);
    let (state, _) = start(&app, "/").await;
    let response = get(
        &app,
        LEADERS_ME_PATH,
        Some(&format!("uma_leader_session={state}")),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sign_out_clears_the_session() {
    let server = MockServer::start().await;
    let app = app(&server);
    let response = app
        .clone()
        .oneshot(Request::post(SIGN_OUT_PATH).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cleared = set_cookies(&response).remove(0);
    assert!(cleared.starts_with("uma_leader_session=;"));
    assert!(cleared.contains("Max-Age=0"));
    // Signing out is a POST, so a link or image on another site can't trigger it.
    let response = get(&app, SIGN_OUT_PATH, None).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn opting_in_isnt_served_without_it_being_set_up() {
    let server = MockServer::start().await;
    let app = app(&server);
    let response = get(&app, &format!("{OPT_IN_PATH}?club=1"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
