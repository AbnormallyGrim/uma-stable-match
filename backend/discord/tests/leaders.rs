//! Leader access tests against a mock Discord and Redis. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, ClubLookup, ClubStore, Discord, DiscordConfig,
    OptIn, PublicKey, PublicUrl, SettingsUpdate, SigningKey, CALLBACK_PATH, CLUBS_PATH, LOGIN_PATH,
    SETTINGS_PATH, SIGN_OUT_PATH, UNLINK_PATH,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Manages server A.
const MANAGER: u64 = 8001;
/// Holds club 1's leader role on server A.
const OFFICER: u64 = 8002;
/// A member of server A with no roles.
const MEMBER: u64 = 8003;
/// Not a member of server A.
const STRANGER: u64 = 8004;

struct World {
    mock: MockServer,
    app: Router,
    store: ClubStore,
    base: u64,
}

impl World {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("REDIS_TEST_URL") else {
            eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
            return None;
        };
        static CALLS: AtomicU64 = AtomicU64::new(0);
        let run = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            % 1_000_000_000;
        let base = (run * 1_000 + CALLS.fetch_add(1, Ordering::Relaxed)) * 100;

        let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
            .await
            .expect("REDIS_TEST_URL is reachable");
        let store = ClubStore::new(conn);
        let mock = MockServer::start().await;
        let mut config = DiscordConfig::new(
            ApplicationId::parse("1000").unwrap(),
            PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
                .unwrap(),
            BotToken::parse("made-up-bot-token").unwrap(),
            ClientSecret::parse("made-up-client-secret").unwrap(),
            SigningKey::parse("made-up-signing-key-of-32-chars!").unwrap(),
            PublicUrl::parse("https://example.test").unwrap(),
        );
        config.api_base = mock.uri();
        let lookup = ClubLookup::new(|_| async { Ok(None) });
        let discord = Discord::new(config)
            .unwrap()
            .with_opt_in(OptIn::new(store.clone(), lookup));
        let world = Self {
            mock,
            app: router(discord),
            store,
            base,
        };
        world.mock_discord().await;
        world.link_clubs().await;
        Some(world)
    }

    fn server_a(&self) -> u64 {
        self.base + 1
    }
    fn server_b(&self) -> u64 {
        self.base + 2
    }
    fn club(&self, n: u64) -> u64 {
        self.base + 10 + n
    }
    fn manager_role(&self) -> u64 {
        self.server_a() + 1000
    }
    fn officers_role(&self) -> u64 {
        self.server_a() + 1001
    }

    /// Server A has clubs 1 (leader role: officers), 2 (none) and 3 (officers, but another
    /// server's); server B, which the bot can't see, has club 4.
    async fn link_clubs(&self) {
        let a = self.server_a();
        for n in 1..=3 {
            self.store.link(self.club(n), a).await.unwrap();
        }
        self.store
            .link(self.club(4), self.server_b())
            .await
            .unwrap();
        for n in [1, 3] {
            let update = SettingsUpdate {
                leader_roles: Some(vec![self.officers_role()]),
                ..Default::default()
            };
            self.store.update(self.club(n), a, &update).await.unwrap();
        }
        // Club 3 moves to server B: it's no longer on server A.
        self.store
            .link(self.club(3), self.server_b())
            .await
            .unwrap();
    }

    async fn mock_discord(&self) {
        for (user, code) in [
            (MANAGER, "manager"),
            (OFFICER, "officer"),
            (MEMBER, "member"),
            (STRANGER, "stranger"),
        ] {
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .and(body_string_contains(format!("code={code}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    json!({ "access_token": format!("token-{code}"), "token_type": "Bearer" }),
                ))
                .mount(&self.mock)
                .await;
            Mock::given(method("GET"))
                .and(path("/users/@me"))
                .and(header("authorization", format!("Bearer token-{code}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({ "id": user.to_string() })),
                )
                .mount(&self.mock)
                .await;
        }
        let a = self.server_a();
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{a}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "owner_id": "999",
                "roles": [
                    { "id": a.to_string(), "name": "@everyone", "permissions": "0" },
                    { "id": self.manager_role().to_string(), "name": "Managers", "permissions": "32" },
                    { "id": self.officers_role().to_string(), "name": "Officers", "permissions": "0" },
                ],
            })))
            .mount(&self.mock)
            .await;
        for (user, roles) in [
            (MANAGER, vec![self.manager_role()]),
            (OFFICER, vec![self.officers_role()]),
            (MEMBER, vec![]),
        ] {
            let roles: Vec<String> = roles.iter().map(u64::to_string).collect();
            Mock::given(method("GET"))
                .and(path(format!("/guilds/{a}/members/{user}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "roles": roles })))
                .mount(&self.mock)
                .await;
        }
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{a}/members/{STRANGER}")))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(json!({ "message": "Unknown Member", "code": 10007 })),
            )
            .mount(&self.mock)
            .await;
        // The bot has been removed from server B.
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{}", self.server_b())))
            .respond_with(ResponseTemplate::new(403))
            .mount(&self.mock)
            .await;
    }

    async fn send(&self, method: &str, uri: &str, cookie: &str) -> Response<Body> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header(COOKIE, cookie)
            .body(Body::empty())
            .unwrap();
        self.app.clone().oneshot(request).await.unwrap()
    }

    /// A session cookie, from signing in with Discord's `code`.
    async fn sign_in(&self, code: &str) -> String {
        let login = self.send("GET", LOGIN_PATH, "").await;
        let location = login.headers()[LOCATION].to_str().unwrap().to_owned();
        let state = reqwest::Url::parse(&location)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned();
        let nonce = cookie(&login, "uma_discord_nonce").unwrap();
        let response = self
            .send(
                "GET",
                &format!("{CALLBACK_PATH}?code={code}&state={state}"),
                &nonce,
            )
            .await;
        cookie(&response, "uma_session").unwrap()
    }

    fn club_uri(&self, n: u64) -> String {
        UNLINK_PATH.replace("{club}", &self.club(n).to_string())
    }

    async fn access(&self, cookies: &str, n: u64) -> Response<Body> {
        self.send("GET", &self.club_uri(n), cookies).await
    }

    async fn clubs(&self, cookies: &str) -> Value {
        let response = self.send("GET", CLUBS_PATH, cookies).await;
        assert_eq!(response.status(), StatusCode::OK);
        json_body(response).await
    }

    async fn clean_up(&self) {
        for n in 1..=4 {
            self.store.unlink(self.club(n)).await.unwrap();
        }
    }
}

/// `name=value` of the cookie set under `name`, if any.
fn cookie(response: &Response<Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|c| c.to_str().unwrap())
        .find(|c| c.starts_with(&format!("{name}=")))
        .map(|c| c.split(';').next().unwrap().to_owned())
}

async fn json_body(response: Response<Body>) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn a_manager_has_manage_access_to_every_club_on_the_server() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in("manager").await;
    for n in [1, 2] {
        let response = world.access(&session, n).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let body = json_body(response).await;
        assert_eq!(body["access"], "manage");
        assert_eq!(body["club"], world.club(n).to_string());
        assert_eq!(body["guild_id"], world.server_a().to_string());
    }
    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_role_holder_can_view_only_the_clubs_that_name_their_role() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in("officer").await;
    let body = json_body(world.access(&session, 1).await).await;
    assert_eq!(body["access"], "view");
    // Club 2 names no role, so being an officer doesn't reach it.
    assert_eq!(
        world.access(&session, 2).await.status(),
        StatusCode::FORBIDDEN
    );
    world.clean_up().await;
}

#[tokio::test]
async fn members_without_a_role_and_strangers_have_no_access() {
    let Some(world) = World::new().await else {
        return;
    };
    for code in ["member", "stranger"] {
        let session = world.sign_in(code).await;
        let response = world.access(&session, 1).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{code}");
        assert!(cookie(&response, "uma_guild").is_none(), "{code}");
    }
    world.clean_up().await;
}

#[tokio::test]
async fn access_needs_a_session_a_club_id_and_a_linked_club_the_bot_can_see() {
    let Some(world) = World::new().await else {
        return;
    };
    assert_eq!(world.access("", 1).await.status(), StatusCode::UNAUTHORIZED);
    let session = world.sign_in("manager").await;
    let bad = UNLINK_PATH.replace("{club}", "abc");
    assert_eq!(
        world.send("GET", &bad, &session).await.status(),
        StatusCode::BAD_REQUEST
    );
    let unlinked = UNLINK_PATH.replace("{club}", &(world.base + 99).to_string());
    assert_eq!(
        world.send("GET", &unlinked, &session).await.status(),
        StatusCode::NOT_FOUND
    );
    // Club 4's server has dropped the bot, so nothing proves anyone's rights.
    assert_eq!(
        world.access(&session, 4).await.status(),
        StatusCode::FORBIDDEN
    );
    world.clean_up().await;
}

#[tokio::test]
async fn getting_access_remembers_the_server_and_the_list_shows_what_each_person_can_see() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in("manager").await;
    // Nothing is remembered yet.
    let empty = world.clubs(&session).await;
    assert_eq!(empty, json!({ "guild_id": null, "clubs": [] }));

    let response = world.access(&session, 1).await;
    let hint = cookie(&response, "uma_guild").expect("a remembered server");
    let set = response.headers().get_all(SET_COOKIE);
    assert!(set
        .iter()
        .any(|c| c.to_str().unwrap().starts_with("uma_guild=")
            && c.to_str().unwrap().contains("HttpOnly")));

    let all = world.clubs(&format!("{session}; {hint}")).await;
    assert_eq!(all["guild_id"], world.server_a().to_string());
    assert_eq!(
        all["clubs"],
        json!([
            { "id": world.club(1).to_string(), "access": "manage" },
            { "id": world.club(2).to_string(), "access": "manage" },
        ])
    );

    // A leader role holder with the same browser's hint sees only their club, as a viewer.
    let officer = world.sign_in("officer").await;
    let officers_own = {
        let response = world.access(&officer, 1).await;
        cookie(&response, "uma_guild").unwrap()
    };
    let some = world.clubs(&format!("{officer}; {officers_own}")).await;
    assert_eq!(
        some["clubs"],
        json!([{ "id": world.club(1).to_string(), "access": "view" }])
    );
    world.clean_up().await;
}

#[tokio::test]
async fn someone_elses_remembered_server_shows_nothing() {
    let Some(world) = World::new().await else {
        return;
    };
    let manager = world.sign_in("manager").await;
    let hint = cookie(&world.access(&manager, 1).await, "uma_guild").unwrap();
    // The same browser, now signed in as someone else, and a tampered hint.
    let stranger = world.sign_in("stranger").await;
    let shown = world.clubs(&format!("{stranger}; {hint}")).await;
    assert_eq!(shown, json!({ "guild_id": null, "clubs": [] }));
    let tampered = format!("{hint}x");
    let shown = world.clubs(&format!("{manager}; {tampered}")).await;
    assert_eq!(shown, json!({ "guild_id": null, "clubs": [] }));
    world.clean_up().await;
}

#[tokio::test]
async fn losing_your_rights_empties_the_list_at_once() {
    let Some(world) = World::new().await else {
        return;
    };
    // A hint made when the person could see the club, replayed for a person Discord now says has
    // no rights: the officer's hint with the member's session isn't valid, so use a club whose
    // role was dropped meanwhile.
    let officer = world.sign_in("officer").await;
    let hint = cookie(&world.access(&officer, 1).await, "uma_guild").unwrap();
    let update = SettingsUpdate {
        leader_roles: Some(vec![]),
        ..Default::default()
    };
    world
        .store
        .update(world.club(1), world.server_a(), &update)
        .await
        .unwrap();
    let shown = world.clubs(&format!("{officer}; {hint}")).await;
    assert_eq!(shown["clubs"], json!([]));
    assert_eq!(
        world.access(&officer, 1).await.status(),
        StatusCode::FORBIDDEN
    );
    world.clean_up().await;
}

#[tokio::test]
async fn the_list_needs_a_session() {
    let Some(world) = World::new().await else {
        return;
    };
    assert_eq!(
        world.send("GET", CLUBS_PATH, "").await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn signing_out_forgets_the_remembered_server() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in("manager").await;
    let response = world.send("POST", SIGN_OUT_PATH, &session).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cleared: Vec<&str> = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|c| c.to_str().unwrap())
        .collect();
    assert!(cleared.iter().any(|c| c.starts_with("uma_session=;")));
    assert!(cleared.iter().any(|c| c.starts_with("uma_guild=;")));
}

#[tokio::test]
async fn a_leader_role_doesnt_open_settings_or_unlinking() {
    let Some(world) = World::new().await else {
        return;
    };
    let officer = world.sign_in("officer").await;
    let settings = SETTINGS_PATH.replace("{club}", &world.club(1).to_string());
    assert_eq!(
        world.send("GET", &settings, &officer).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        world.send("PATCH", &settings, &officer).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        world
            .send("DELETE", &world.club_uri(1), &officer)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    // Nothing was unlinked.
    assert!(world.store.club(world.club(1)).await.unwrap().is_some());
    world.clean_up().await;
}
