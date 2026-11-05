//! Club opt-in tests against a mock Discord and Redis. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::json;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, ClubLookup, ClubStore, ClubSummary, Discord,
    DiscordConfig, KeySecret, LookupError, OptIn, PublicKey, PublicUrl, SigningKey, TrainerLinks,
    TrainerLookup, TrainerStore, TrainerSummary, BOT_PERMISSIONS, CALLBACK_PATH, LOGIN_PATH,
    MY_CLUB_PATH, OPT_IN_PATH, SETTINGS_PATH, UNLINK_PATH,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A mock Discord, an app using it, a fake uma.moe and Redis stores, with IDs no other test uses.
///
/// Three people sign in. The leader manages server A and has linked the trainer that leads clubs
/// 1 to 8. The other user is a member of server A without Manage Server, and has linked the
/// trainer that leads club 9. The stranger is a member of server A with no trainer linked.
struct World {
    mock: MockServer,
    app: Router,
    store: ClubStore,
    trainers: TrainerStore,
    /// Each club's leader, by trainer ID.
    leaders: Arc<Mutex<HashMap<u64, u64>>>,
    club_lookups: Arc<AtomicUsize>,
    /// What the bot's role on server A grants, for the mock to answer with.
    bot_role: Arc<AtomicU64>,
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
        let store = ClubStore::new(conn.clone());
        let trainers = TrainerStore::new(
            conn,
            KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap(),
        )
        .with_check_queue(&format!("test:checks:{base}"));
        let mock = MockServer::start().await;
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
        config.api_base = mock.uri();

        // Clubs: base+11..13 point to server A, 14 has an expired invite, 15 has none, 16 points
        // to A and B, 17 can't be looked up, 18 points to B, 19 points to A.
        let descriptions: HashMap<u64, Option<String>> = HashMap::from([
            (base + 11, Some("discord.gg/aaa111".into())),
            (base + 12, Some("Discord: discord.gg/aaa111".into())),
            (base + 13, Some("discord.com/invite/aaa111".into())),
            (base + 14, Some("discord.gg/old123".into())),
            (base + 15, None),
            (
                base + 16,
                Some("discord.gg/aaa111 discord.gg/bbb222".into()),
            ),
            (base + 18, Some("discord.gg/bbb222".into())),
            (base + 19, Some("discord.gg/aaa111".into())),
        ]);
        let unavailable = base + 17;
        let leaders: Arc<Mutex<HashMap<u64, u64>>> = Arc::new(Mutex::new(
            (11..=18)
                .map(|n| (base + n, base + 50))
                .chain([(base + 19, base + 51)])
                .collect(),
        ));
        let club_lookups = Arc::new(AtomicUsize::new(0));
        let lookup = {
            let (leaders, club_lookups) = (leaders.clone(), club_lookups.clone());
            ClubLookup::new(move |id| {
                club_lookups.fetch_add(1, Ordering::Relaxed);
                let found = descriptions.get(&id).cloned();
                let leader = leaders.lock().unwrap().get(&id).copied();
                async move {
                    if id == unavailable {
                        return Err(LookupError);
                    }
                    Ok(found.map(|description| ClubSummary {
                        name: "Example Club".into(),
                        description,
                        leader,
                    }))
                }
            })
        };
        // Trainers base+50 and base+52 are in club 1, base+51 in club 9.
        let profiles: HashMap<u64, u64> = HashMap::from([
            (base + 50, base + 11),
            (base + 51, base + 19),
            (base + 52, base + 11),
        ]);
        let profile = TrainerLookup::new(move |id| {
            let club = profiles.get(&id).copied();
            async move {
                Ok(club.map(|club| TrainerSummary {
                    name: "Example Trainer".into(),
                    comment: None,
                    club: Some(club),
                }))
            }
        });
        let discord = Discord::new(config)
            .unwrap()
            .with_opt_in(OptIn::new(store.clone(), lookup))
            .with_trainer_links(TrainerLinks::new(trainers.clone(), profile));
        let world = Self {
            mock,
            app: router(discord),
            store,
            trainers,
            leaders,
            club_lookups,
            bot_role: Arc::new(AtomicU64::new(BOT_PERMISSIONS)),
            base,
        };
        world.mock_discord().await;
        for (user, trainer) in [
            (world.leader(), world.leader_trainer()),
            (world.other_user(), world.other_trainer()),
        ] {
            world
                .trainers
                .link(user, trainer, "Example Trainer")
                .await
                .unwrap();
        }
        Some(world)
    }

    fn leader(&self) -> u64 {
        self.base + 61
    }
    fn other_user(&self) -> u64 {
        self.base + 62
    }
    fn stranger(&self) -> u64 {
        self.base + 63
    }
    fn leader_trainer(&self) -> u64 {
        self.base + 50
    }
    fn other_trainer(&self) -> u64 {
        self.base + 51
    }
    fn stranger_trainer(&self) -> u64 {
        self.base + 52
    }

    /// Makes `trainer` the leader of club `n`.
    fn set_leader(&self, n: u64, trainer: u64) {
        self.leaders.lock().unwrap().insert(self.club(n), trainer);
    }

    fn server_a(&self) -> u64 {
        self.base + 1
    }
    fn server_b(&self) -> u64 {
        self.base + 2
    }
    fn old_server(&self) -> u64 {
        self.base + 3
    }
    fn club(&self, n: u64) -> u64 {
        self.base + 10 + n
    }

    async fn mock_discord(&self) {
        let token = |code: &str, access: &str, guild: Option<u64>| {
            let mut body = json!({ "access_token": access, "token_type": "Bearer" });
            if let Some(guild) = guild {
                body["guild"] = json!({ "id": guild.to_string() });
            }
            (code.to_owned(), body)
        };
        for (code, body) in [
            token("leader-code", "leader-token", None),
            token("other-code", "other-token", None),
            token("stranger-code", "stranger-token", None),
            token("install-a", "leader-token", Some(self.server_a())),
            token("install-b", "leader-token", Some(self.server_b())),
            token("install-nothing", "leader-token", None),
            token("install-other-user", "other-token", Some(self.server_a())),
            token("install-stranger", "stranger-token", Some(self.server_a())),
        ] {
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .and(body_string_contains(format!("code={code}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&self.mock)
                .await;
        }
        for (access, id) in [
            ("leader-token", self.leader()),
            ("other-token", self.other_user()),
            ("stranger-token", self.stranger()),
        ] {
            Mock::given(method("GET"))
                .and(path("/users/@me"))
                .and(header("authorization", format!("Bearer {access}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({ "id": id.to_string() })),
                )
                .mount(&self.mock)
                .await;
        }
        for (code, guild) in [
            ("aaa111", Some(self.server_a())),
            ("bbb222", Some(self.server_b())),
            ("old123", None),
        ] {
            let response = match guild {
                Some(guild) => ResponseTemplate::new(200)
                    .set_body_json(json!({ "code": code, "guild": { "id": guild.to_string() } })),
                None => ResponseTemplate::new(404)
                    .set_body_json(json!({ "message": "Unknown Invite", "code": 10006 })),
            };
            Mock::given(method("GET"))
                .and(path(format!("/invites/{code}")))
                .respond_with(response)
                .mount(&self.mock)
                .await;
        }
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.mock)
            .await;

        // Server A: someone else owns it; one role has Manage Server, and `@everyone` (the role
        // with the server's ID) has nothing. The leader has that role, the other user doesn't. The
        // bot, whose ID is the application's, has a role granting `bot_role`.
        let a = self.server_a();
        let manager_role = a + 1000;
        let bot_role = a + 1002;
        let granted = self.bot_role.clone();
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{a}")))
            .respond_with(move |_: &wiremock::Request| {
                ResponseTemplate::new(200).set_body_json(json!({
                    "owner_id": "999",
                    "roles": [
                        { "id": a.to_string(), "name": "@everyone", "permissions": "0" },
                        { "id": manager_role.to_string(), "name": "Managers", "permissions": "32" },
                        { "id": (a + 1001).to_string(), "name": "officers", "permissions": "0" },
                        {
                            "id": bot_role.to_string(),
                            "name": "Bot",
                            "permissions": granted.load(Ordering::Relaxed).to_string(),
                        },
                    ],
                }))
            })
            .mount(&self.mock)
            .await;
        for (user, roles) in [
            (self.leader(), vec![manager_role.to_string()]),
            (self.other_user(), vec![]),
            (self.stranger(), vec![]),
            (1000, vec![bot_role.to_string()]),
        ] {
            Mock::given(method("GET"))
                .and(path(format!("/guilds/{a}/members/{user}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "roles": roles })))
                .mount(&self.mock)
                .await;
        }
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{a}/channels")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "id": (a + 2000).to_string(), "name": "Applications", "type": 4 },
                { "id": (a + 2001).to_string(), "name": "general", "type": 0 },
                { "id": (a + 2002).to_string(), "name": "archive", "type": 4 },
            ])))
            .mount(&self.mock)
            .await;
        // The bot was kicked from the old server.
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{}/channels", self.old_server())))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_json(json!({ "message": "Missing Access", "code": 50001 })),
            )
            .mount(&self.mock)
            .await;
        // Server B is down.
        Mock::given(method("GET"))
            .and(path(format!("/guilds/{}", self.server_b())))
            .respond_with(ResponseTemplate::new(503))
            .mount(&self.mock)
            .await;
    }

    async fn get(&self, uri: &str, cookie: &str) -> Response<Body> {
        self.send("GET", uri, cookie).await
    }

    async fn send(&self, method: &str, uri: &str, cookie: &str) -> Response<Body> {
        self.send_body(method, uri, cookie, "").await
    }

    async fn send_body(&self, method: &str, uri: &str, cookie: &str, body: &str) -> Response<Body> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header(COOKIE, cookie);
        self.app
            .clone()
            .oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap()
    }

    /// A session cookie, from signing in as the leader.
    async fn sign_in(&self) -> String {
        self.sign_in_with("leader-code").await
    }

    async fn sign_in_with(&self, code: &str) -> String {
        let login = self.get(LOGIN_PATH, "").await;
        let state = query(&location(&login), "state");
        let nonce = cookie(&login, "uma_discord_nonce");
        let uri = format!("{CALLBACK_PATH}?code={code}&state={state}");
        let response = self.get(&uri, &nonce).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        cookie(&response, "uma_session")
    }

    async fn start(&self, session: &str, club: u64) -> Response<Body> {
        self.get(
            &format!("{OPT_IN_PATH}?club={club}&next=/club/{club}"),
            session,
        )
        .await
    }

    /// Finishes an opt-in the way Discord's redirect would, with `extra` added to the query.
    async fn finish(&self, started: &Response<Body>, code: &str, extra: &str) -> Response<Body> {
        let state = query(&location(started), "state");
        let nonce = cookie(started, "uma_discord_nonce");
        let uri = format!("{CALLBACK_PATH}?code={code}&state={state}{extra}");
        self.get(&uri, &nonce).await
    }

    async fn unlink(&self, session: &str, club: u64) -> Response<Body> {
        let uri = UNLINK_PATH.replace("{club}", &club.to_string());
        self.send("DELETE", &uri, session).await
    }

    fn settings_uri(&self, club: u64) -> String {
        SETTINGS_PATH.replace("{club}", &club.to_string())
    }

    async fn settings(&self, session: &str, club: u64) -> Response<Body> {
        self.get(&self.settings_uri(club), session).await
    }

    async fn change(&self, session: &str, club: u64, body: serde_json::Value) -> Response<Body> {
        let uri = self.settings_uri(club);
        self.send_body("PATCH", &uri, session, &body.to_string())
            .await
    }

    async fn left(&self) -> Vec<String> {
        self.mock
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.method.as_str() == "DELETE")
            .map(|request| request.url.path().to_owned())
            .collect()
    }

    async fn my_club(&self, session: &str) -> Response<Body> {
        self.get(MY_CLUB_PATH, session).await
    }

    async fn clean_up(&self) {
        for n in 1..=9 {
            self.store.unlink(self.club(n)).await.unwrap();
        }
        self.store.unlink(self.base + 20).await.unwrap();
        for user in [self.leader(), self.other_user(), self.stranger()] {
            self.trainers.forget(user).await.unwrap();
        }
    }
}

fn location(response: &Response<Body>) -> String {
    response.headers()[LOCATION].to_str().unwrap().to_owned()
}

fn query(location: &str, name: &str) -> String {
    let url = reqwest::Url::parse(location).unwrap();
    let (_, value) = url.query_pairs().find(|(k, _)| k == name).unwrap();
    value.into_owned()
}

/// `name=value` of the cookie set under `name`.
fn cookie(response: &Response<Body>, name: &str) -> String {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|c| c.to_str().unwrap())
        .find(|c| c.starts_with(&format!("{name}=")))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn starting_needs_a_session_a_real_club_and_one_working_invite() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let status = |response: Response<Body>| response.status();

    assert_eq!(
        status(world.start("", world.club(1)).await),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(
            world
                .get(&format!("{OPT_IN_PATH}?club=abc"), &session)
                .await
        ),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status(world.start(&session, world.base + 99).await),
        StatusCode::NOT_FOUND
    );
    for club in [4, 5] {
        // An expired invite, and no invite at all.
        assert_eq!(
            status(world.start(&session, world.club(club)).await),
            StatusCode::UNPROCESSABLE_ENTITY,
            "club {club}"
        );
    }
    // Invites to two servers.
    assert_eq!(
        status(world.start(&session, world.club(6)).await),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    // The club can't be looked up.
    assert_eq!(
        status(world.start(&session, world.club(7)).await),
        StatusCode::BAD_GATEWAY
    );
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_clubs_leader_can_start() {
    let Some(world) = World::new().await else {
        return;
    };
    // No trainer linked.
    let stranger = world.sign_in_with("stranger-code").await;
    let response = world.start(&stranger, world.club(1)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // A linked trainer that doesn't lead the club.
    let other = world.sign_in_with("other-code").await;
    let response = world.start(&other, world.club(1)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // The leader is checked before the invite, so a non-leader learns nothing about it.
    let response = world.start(&other, world.club(5)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = world.start(&other, world.club(9)).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    world.clean_up().await;
}

#[tokio::test]
async fn starting_sends_the_leader_to_add_the_bot_to_the_clubs_server() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let response = world.start(&session, world.club(1)).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let to = location(&response);
    assert!(to.starts_with("https://discord.com/oauth2/authorize?"));
    assert_eq!(query(&to, "scope"), "bot identify");
    assert_eq!(query(&to, "response_type"), "code");
    assert_eq!(query(&to, "guild_id"), world.server_a().to_string());
    assert_eq!(query(&to, "disable_guild_select"), "true");
    assert_eq!(query(&to, "permissions"), BOT_PERMISSIONS.to_string());
    assert!(cookie(&response, "uma_discord_nonce").len() > "uma_discord_nonce=".len());
    // Starting links nothing.
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn adding_the_bot_to_the_invites_server_links_the_club() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-a", "").await;

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(location(&response), format!("/club/{}", world.club(1)));
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert_eq!(record.guild_id, world.server_a());
    assert!(world.left().await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn adding_the_bot_to_another_server_refuses_and_leaves_it() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-b", "").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    assert!(world
        .store
        .clubs_on(world.server_b())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.server_b())]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn editing_the_callbacks_query_cant_link_another_server() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    // The bot went to B, but the query claims A, with every permission.
    let claim = format!("&guild_id={}&permissions=8", world.server_a());
    let response = world.finish(&started, "install-b", &claim).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn the_bot_stays_in_a_wrong_server_another_club_uses() {
    let Some(world) = World::new().await else {
        return;
    };
    // Club 8 already uses server B.
    world
        .store
        .link(world.club(8), world.server_b())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-b", "").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(world.left().await.is_empty());
    assert_eq!(
        world.store.clubs_on(world.server_b()).await.unwrap(),
        [world.club(8)]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_second_club_on_the_same_server_links_without_affecting_the_first() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    for (club, expected) in [(1, vec![1]), (2, vec![1, 2]), (3, vec![1, 2, 3])] {
        let started = world.start(&session, world.club(club)).await;
        let response = world.finish(&started, "install-a", "").await;
        assert_eq!(
            response.status(),
            StatusCode::TEMPORARY_REDIRECT,
            "club {club}"
        );
        let clubs: Vec<u64> = expected.iter().map(|n| world.club(*n)).collect();
        assert_eq!(world.store.clubs_on(world.server_a()).await.unwrap(), clubs);
    }
    assert!(world.left().await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn moving_a_club_leaves_the_old_server_when_no_club_uses_it() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.old_server())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-a", "").await;

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert_eq!(record.guild_id, world.server_a());
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.old_server())]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn moving_a_club_keeps_the_old_server_when_a_sibling_uses_it() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.old_server())
        .await
        .unwrap();
    world
        .store
        .link(world.club(2), world.old_server())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    world.finish(&started, "install-a", "").await;

    assert!(world.left().await.is_empty());
    assert_eq!(
        world.store.clubs_on(world.old_server()).await.unwrap(),
        [world.club(2)]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_without_manage_server_cant_add_the_bot() {
    let Some(world) = World::new().await else {
        return;
    };
    // The other user leads club 9, whose invite points to server A, but can't manage A.
    let session = world.sign_in_with("other-code").await;
    let started = world.start(&session, world.club(9)).await;
    let response = world.finish(&started, "install-other-user", "").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(world.store.club(world.club(9)).await.unwrap(), None);
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.server_a())]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn the_bot_added_without_every_permission_links_nothing_and_leaves() {
    let Some(world) = World::new().await else {
        return;
    };
    // The leader unticked Manage Channels on Discord's page.
    world
        .bot_role
        .store(BOT_PERMISSIONS & !(1 << 4), Ordering::Relaxed);
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-a", "").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.server_a())]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn administrator_covers_every_permission_the_bot_needs() {
    let Some(world) = World::new().await else {
        return;
    };
    world.bot_role.store(1 << 3, Ordering::Relaxed);
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-a", "").await;

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert!(world.store.club(world.club(1)).await.unwrap().is_some());
    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_who_steps_down_before_finishing_links_nothing() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    world.set_leader(1, world.other_trainer());
    let response = world.finish(&started, "install-a", "").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.server_a())]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn another_person_cant_finish_someone_elses_opt_in() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-other-user", "").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn declining_to_add_the_bot_links_nothing() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let response = world.finish(&started, "install-nothing", "").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    assert!(world.left().await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn the_nonce_still_has_to_match() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in().await;
    let started = world.start(&session, world.club(1)).await;
    let state = query(&location(&started), "state");
    let uri = format!("{CALLBACK_PATH}?code=install-a&state={state}");
    let response = world.get(&uri, "").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn a_server_manager_unlinks_a_club_and_the_bot_leaves_an_unused_server() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let response = world.unlink(&session, world.club(1)).await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(world.store.club(world.club(1)).await.unwrap(), None);
    assert!(world
        .store
        .clubs_on(world.server_a())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        world.left().await,
        [format!("/users/@me/guilds/{}", world.server_a())]
    );
    // Unlinking again finds nothing.
    let again = world.unlink(&session, world.club(1)).await;
    assert_eq!(again.status(), StatusCode::NOT_FOUND);
    world.clean_up().await;
}

#[tokio::test]
async fn unlinking_one_club_keeps_the_bot_while_a_sibling_uses_the_server() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    world
        .store
        .link(world.club(2), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let response = world.unlink(&session, world.club(1)).await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(world.left().await.is_empty());
    assert_eq!(
        world.store.clubs_on(world.server_a()).await.unwrap(),
        [world.club(2)]
    );
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_clubs_leader_can_unlink() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    for code in ["other-code", "stranger-code"] {
        let session = world.sign_in_with(code).await;
        let response = world.unlink(&session, world.club(1)).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{code}");
    }
    assert!(world.store.club(world.club(1)).await.unwrap().is_some());
    assert!(world.left().await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn unlinking_needs_a_session_a_club_id_and_a_linked_club() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;

    let without = world.unlink("", world.club(1)).await;
    assert_eq!(without.status(), StatusCode::UNAUTHORIZED);
    let bad = world
        .send("DELETE", "/api/v1/discord/clubs/abc", &session)
        .await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    let unlinked = world.unlink(&session, world.club(2)).await;
    assert_eq!(unlinked.status(), StatusCode::NOT_FOUND);
    // A link needs a DELETE.
    let uri = UNLINK_PATH.replace("{club}", &world.club(1).to_string());
    let get = world.get(&uri, &session).await;
    assert_eq!(get.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(world.store.club(world.club(1)).await.unwrap().is_some());
    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_unlinks_from_a_server_the_bot_left_but_not_during_an_outage() {
    let Some(world) = World::new().await else {
        return;
    };
    // The old server isn't known to the mock, as if the bot had been removed from it.
    world
        .store
        .link(world.club(1), world.old_server())
        .await
        .unwrap();
    // Club 7 can't be looked up, so its leader can't be checked.
    world
        .store
        .link(world.club(7), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;

    let gone = world.unlink(&session, world.club(1)).await;
    assert_eq!(gone.status(), StatusCode::NO_CONTENT);
    let outage = world.unlink(&session, world.club(7)).await;
    assert_eq!(outage.status(), StatusCode::BAD_GATEWAY);
    assert!(world.store.club(world.club(7)).await.unwrap().is_some());
    world.clean_up().await;
}

async fn json_body(response: Response<Body>) -> serde_json::Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn the_leader_reads_the_clubs_settings_with_the_servers_categories() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let response = world.settings(&session, world.club(1)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let a = world.server_a();
    assert_eq!(
        json_body(response).await,
        json!({
            "guild_id": a.to_string(),
            "category": null,
            "auto_join": true,
            "open": true,
            "problem": null,
            "categories": [
                { "id": (a + 2000).to_string(), "name": "Applications" },
                { "id": (a + 2002).to_string(), "name": "archive" },
            ],
        })
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_kicked_bot_unlinks_the_servers_clubs_so_the_bot_can_be_added_again() {
    let Some(world) = World::new().await else {
        return;
    };
    for n in [2, 3] {
        world
            .store
            .link(world.club(n), world.old_server())
            .await
            .unwrap();
    }
    let session = world.sign_in().await;
    let response = world.settings(&session, world.club(2)).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    // Every club on the server went, as the bot is gone for all of them.
    assert!(world
        .store
        .clubs_on(world.old_server())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(world.store.club(world.club(2)).await.unwrap(), None);
    assert_eq!(world.store.club(world.club(3)).await.unwrap(), None);
    // Changing settings of a kicked bot's club does the same.
    world
        .store
        .link(world.club(2), world.old_server())
        .await
        .unwrap();
    let response = world
        .change(&session, world.club(2), json!({ "open": false }))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(world.store.club(world.club(2)).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_clubs_leader_reads_or_changes_settings() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let a = world.server_a();
    let patch = json!({ "open": false, "category": (a + 2000).to_string() });

    let anonymous = world.settings("", world.club(1)).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let anonymous = world.change("", world.club(1), patch.clone()).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    // Signed in, but not the club's leader.
    for code in ["other-code", "stranger-code"] {
        let session = world.sign_in_with(code).await;
        let read = world.settings(&session, world.club(1)).await;
        assert_eq!(read.status(), StatusCode::FORBIDDEN, "{code}");
        let change = world.change(&session, world.club(1), patch.clone()).await;
        assert_eq!(change.status(), StatusCode::FORBIDDEN, "{code}");
    }
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert!(record.open && record.category.is_none());

    // Managing the server isn't enough: the leader manages server A but doesn't lead club 9.
    world
        .store
        .link(world.club(9), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let read = world.settings(&session, world.club(9)).await;
    assert_eq!(read.status(), StatusCode::FORBIDDEN);

    // And a club that isn't linked has nothing to read.
    let missing = world.settings(&session, world.club(2)).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    world.clean_up().await;
}

#[tokio::test]
async fn every_read_and_change_checks_the_leader_again() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let session = world.sign_in().await;
    let before = world.club_lookups.load(Ordering::Relaxed);
    world.settings(&session, world.club(1)).await;
    world.settings(&session, world.club(1)).await;
    world
        .change(&session, world.club(1), json!({ "auto_join": false }))
        .await;
    assert_eq!(world.club_lookups.load(Ordering::Relaxed) - before, 3);

    // A leader who stepped down loses access at once.
    world.set_leader(1, world.other_trainer());
    let read = world.settings(&session, world.club(1)).await;
    assert_eq!(read.status(), StatusCode::FORBIDDEN);
    world.clean_up().await;
}

#[tokio::test]
async fn changing_settings_saves_them_and_answers_with_the_result() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let a = world.server_a();
    let session = world.sign_in().await;
    let response = world
        .change(
            &session,
            world.club(1),
            json!({
                "category": (a + 2000).to_string(),
                "auto_join": false,
                "open": false,
            }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["category"], (a + 2000).to_string());
    assert_eq!(body["auto_join"], false);
    assert_eq!(body["open"], false);
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert_eq!(record.category, Some(a + 2000));
    assert!(!record.auto_join && !record.open);

    // `null` clears the category; fields left out stay.
    let response = world
        .change(&session, world.club(1), json!({ "category": null }))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert_eq!(record.category, None);
    assert!(!record.open);
    world.clean_up().await;
}

#[tokio::test]
async fn settings_must_name_the_servers_own_categories() {
    let Some(world) = World::new().await else {
        return;
    };
    world
        .store
        .link(world.club(1), world.server_a())
        .await
        .unwrap();
    let a = world.server_a();
    let session = world.sign_in().await;
    for (what, patch) in [
        (
            "a text channel",
            json!({ "category": (a + 2001).to_string() }),
        ),
        ("another server's category", json!({ "category": "42" })),
        ("junk", json!({ "category": "abc" })),
        ("leader roles", json!({ "leader_roles": [] })),
        ("nothing", json!({})),
        ("an unknown field", json!({ "problem": "none" })),
        ("a wrong type", json!({ "open": "yes" })),
    ] {
        let response = world.change(&session, world.club(1), patch).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{what}");
    }
    let invalid = world
        .send_body(
            "PATCH",
            &world.settings_uri(world.club(1)),
            &session,
            "not json",
        )
        .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let record = world.store.club(world.club(1)).await.unwrap().unwrap();
    assert!(record.open && record.auto_join && record.category.is_none());
    world.clean_up().await;
}

#[tokio::test]
async fn my_club_says_which_club_the_linked_trainer_is_in_and_whether_it_leads_it() {
    let Some(world) = World::new().await else {
        return;
    };
    assert_eq!(world.my_club("").await.status(), StatusCode::UNAUTHORIZED);

    let stranger = world.sign_in_with("stranger-code").await;
    let response = world.my_club(&stranger).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_body(response).await,
        json!({ "trainer": null, "club": null })
    );

    let leader = world.sign_in().await;
    assert_eq!(
        json_body(world.my_club(&leader).await).await,
        json!({
            "trainer": world.leader_trainer().to_string(),
            "club": { "id": world.club(1).to_string(), "name": "Example Club", "leader": true },
        })
    );

    // A member of club 1 who doesn't lead it.
    world
        .trainers
        .link(
            world.stranger(),
            world.stranger_trainer(),
            "Example Trainer",
        )
        .await
        .unwrap();
    assert_eq!(
        json_body(world.my_club(&stranger).await).await,
        json!({
            "trainer": world.stranger_trainer().to_string(),
            "club": { "id": world.club(1).to_string(), "name": "Example Club", "leader": false },
        })
    );
    world.clean_up().await;
}
