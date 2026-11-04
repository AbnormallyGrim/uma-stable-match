//! Trainer link tests against a mock Discord, a fake uma.moe and Redis. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, COOKIE, LOCATION, RETRY_AFTER, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::json;
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, KeySecret, LookupError,
    PublicKey, PublicUrl, SigningKey, TrainerLinks, TrainerLookup, TrainerStore, TrainerSummary,
    CALLBACK_PATH, LINK_PATH, LOGIN_PATH, MAX_LINKED_TRAINERS, TRAINERS_PATH, TRAINER_PATH,
    VERIFICATION_PATH,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// What the fake uma.moe says about a trainer.
#[derive(Clone)]
enum Profile {
    Comment(&'static str),
    NoComment,
    Down,
}

/// A mock Discord for sign-in, an app with a fake uma.moe and a Redis store, with IDs no other
/// test uses.
struct World {
    app: Router,
    store: TrainerStore,
    profiles: Arc<Mutex<HashMap<u64, Profile>>>,
    lookups: Arc<AtomicUsize>,
    base: u64,
    _mock: MockServer,
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
            % 1_000_000;
        let base = (run * 1_000 + CALLS.fetch_add(1, Ordering::Relaxed)) * 10;

        let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
            .await
            .expect("REDIS_TEST_URL is reachable");
        let store = TrainerStore::new(
            conn,
            KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap(),
        );
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

        let profiles: Arc<Mutex<HashMap<u64, Profile>>> = Arc::default();
        let lookups = Arc::new(AtomicUsize::new(0));
        let lookup = {
            let (profiles, lookups) = (profiles.clone(), lookups.clone());
            TrainerLookup::new(move |id| {
                lookups.fetch_add(1, Ordering::Relaxed);
                let profile = profiles.lock().unwrap().get(&id).cloned();
                async move {
                    match profile {
                        None => Ok(None),
                        Some(Profile::Down) => Err(LookupError),
                        Some(Profile::NoComment) => Ok(Some(TrainerSummary {
                            name: "Example Trainer".into(),
                            comment: None,
                        })),
                        Some(Profile::Comment(comment)) => Ok(Some(TrainerSummary {
                            name: "Example Trainer".into(),
                            comment: Some(comment.to_owned()),
                        })),
                    }
                }
            })
        };
        let discord = Discord::new(config)
            .unwrap()
            .with_trainer_links(TrainerLinks::new(store.clone(), lookup));
        let world = Self {
            app: router(discord),
            store,
            profiles,
            lookups,
            base,
            _mock: mock,
        };
        world.mock_sign_in().await;
        Some(world)
    }

    /// A Discord user, one of three.
    fn user(&self, n: u64) -> u64 {
        self.base + n
    }

    /// A trainer ID, one of twelve.
    fn trainer(&self, n: u64) -> u64 {
        100_000_000_000 + self.base * 10 + n
    }

    fn set(&self, trainer: u64, profile: Profile) {
        self.profiles.lock().unwrap().insert(trainer, profile);
    }

    async fn mock_sign_in(&self) {
        for n in 1..=3 {
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .and(body_string_contains(format!("code=user{n}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    json!({ "access_token": format!("token{n}"), "token_type": "Bearer" }),
                ))
                .mount(&self._mock)
                .await;
            Mock::given(method("GET"))
                .and(path("/users/@me"))
                .and(header("authorization", format!("Bearer token{n}")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json!({ "id": self.user(n).to_string() })),
                )
                .mount(&self._mock)
                .await;
        }
    }

    async fn send(&self, method: &str, uri: &str, cookie: &str) -> Response<Body> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header(COOKIE, cookie)
            .body(Body::empty())
            .unwrap();
        self.app.clone().oneshot(request).await.unwrap()
    }

    /// A session cookie, from signing in as user `n`.
    async fn sign_in(&self, n: u64) -> String {
        let login = self.send("GET", LOGIN_PATH, "").await;
        let location = login.headers()[LOCATION].to_str().unwrap().to_owned();
        let state = reqwest::Url::parse(&location)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned();
        let nonce = cookie(&login, "uma_discord_nonce");
        let uri = format!("{CALLBACK_PATH}?code=user{n}&state={state}");
        let response = self.send("GET", &uri, &nonce).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        cookie(&response, "uma_session")
    }

    fn uri(path: &str, trainer: u64) -> String {
        path.replace("{trainer}", &trainer.to_string())
    }

    async fn list(&self, session: &str) -> Response<Body> {
        self.send("GET", TRAINERS_PATH, session).await
    }

    async fn start(&self, session: &str, trainer: u64) -> Response<Body> {
        self.send("POST", &Self::uri(VERIFICATION_PATH, trainer), session)
            .await
    }

    async fn check(&self, session: &str, trainer: u64) -> Response<Body> {
        self.send("POST", &Self::uri(LINK_PATH, trainer), session)
            .await
    }

    async fn unlink(&self, session: &str, trainer: u64) -> Response<Body> {
        self.send("DELETE", &Self::uri(TRAINER_PATH, trainer), session)
            .await
    }

    /// The key from starting a verification, which must have worked.
    async fn key(&self, session: &str, trainer: u64) -> String {
        let response = self.start(session, trainer).await;
        assert_eq!(response.status(), StatusCode::OK);
        json_body(response).await["key"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// Asks for a key, puts it in the trainer's comment and checks; the check's status.
    async fn prove(&self, session: &str, trainer: u64) -> StatusCode {
        let key = self.key(session, trainer).await;
        self.set(trainer, Profile::Comment(Box::leak(key.into_boxed_str())));
        self.check(session, trainer).await.status()
    }

    async fn linked(&self, session: &str) -> Vec<String> {
        let body = json_body(self.list(session).await).await;
        body["linked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect()
    }

    async fn clean_up(&self) {
        for n in 1..=3 {
            for t in 1..=12 {
                self.store
                    .unlink(self.user(n), self.trainer(t))
                    .await
                    .unwrap();
            }
        }
    }
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

async fn json_body(response: Response<Body>) -> serde_json::Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn every_route_needs_a_session() {
    let Some(world) = World::new().await else {
        return;
    };
    let t = world.trainer(1);
    assert_eq!(world.list("").await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(world.start("", t).await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(world.check("", t).await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(world.unlink("", t).await.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_trainer_id_must_be_nine_to_twelve_digits() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    for bad in ["12345678", "1234567890123", "abc123456", "+12345678901"] {
        for (method, route) in [
            ("POST", VERIFICATION_PATH),
            ("POST", LINK_PATH),
            ("DELETE", TRAINER_PATH),
        ] {
            let uri = route.replace("{trainer}", bad.replace('+', "%2B").as_str());
            let response = world.send(method, &uri, &session).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{method} {bad}");
        }
    }
}

#[tokio::test]
async fn asking_for_a_key_gives_the_same_private_key_until_it_is_used() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    let first = world.start(&session, t).await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(first.headers()[CACHE_CONTROL], "no-store");
    let first = json_body(first).await;
    let key = first["key"].as_str().unwrap();
    assert!(key.starts_with("uma-") && key.len() == 10, "{key}");
    let left = first["expires_in"].as_u64().unwrap();
    assert!((86_000..=86_400).contains(&left), "{left}");

    let again = json_body(world.start(&session, t).await).await;
    assert_eq!(again["key"], first["key"]);
    let other = json_body(world.start(&session, world.trainer(2)).await).await;
    assert_ne!(other["key"], first["key"]);
    world.clean_up().await;
}

#[tokio::test]
async fn checking_needs_a_key_first_and_does_not_call_uma_moe_without_one() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    world.set(t, Profile::Comment("anything"));
    let response = world.check(&session, t).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(world.lookups.load(Ordering::Relaxed), 0);
    world.clean_up().await;
}

#[tokio::test]
async fn the_key_in_the_comment_links_the_trainer() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    let key = world.key(&session, t).await;

    // Not there yet: no comment, then someone else's words.
    world.set(t, Profile::NoComment);
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    world.set(t, Profile::Comment("Looking for a club!"));
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(world.linked(&session).await.is_empty());

    // Now it is, with other text around it and in any case.
    let comment: &'static str = Box::leak(format!("hi {} :)", key.to_uppercase()).into_boxed_str());
    world.set(t, Profile::Comment(comment));
    let response = world.check(&session, t).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let body = json_body(response).await;
    assert_eq!(body, json!({ "linked": true, "name": "Example Trainer" }));
    assert_eq!(world.linked(&session).await, vec![t.to_string()]);

    // The key is used up, and the trainer can't be linked twice.
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        world.start(&session, t).await.status(),
        StatusCode::CONFLICT
    );
    world.clean_up().await;
}

#[tokio::test]
async fn an_unknown_trainer_and_an_outage_link_nothing() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let (unknown, down) = (world.trainer(1), world.trainer(2));
    world.key(&session, unknown).await;
    world.key(&session, down).await;
    world.set(down, Profile::Down);
    assert_eq!(
        world.check(&session, unknown).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        world.check(&session, down).await.status(),
        StatusCode::BAD_GATEWAY
    );
    assert!(world.linked(&session).await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn someone_elses_key_is_no_use() {
    let Some(world) = World::new().await else {
        return;
    };
    let (alice, bob) = (world.sign_in(1).await, world.sign_in(2).await);
    let t = world.trainer(1);
    let alices = world.key(&alice, t).await;
    let bobs = world.key(&bob, t).await;
    assert_ne!(alices, bobs);

    // The profile shows only Alice's key, so Bob can't claim the trainer with it.
    let comment: &'static str = Box::leak(alices.into_boxed_str());
    world.set(t, Profile::Comment(comment));
    assert_eq!(
        world.check(&bob, t).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(world.linked(&bob).await.is_empty());
    assert_eq!(world.check(&alice, t).await.status(), StatusCode::OK);
    world.clean_up().await;
}

#[tokio::test]
async fn proving_it_again_moves_a_trainer_to_the_new_owner() {
    let Some(world) = World::new().await else {
        return;
    };
    let (alice, bob) = (world.sign_in(1).await, world.sign_in(2).await);
    let t = world.trainer(1);
    let alices = world.key(&alice, t).await;
    world.set(t, Profile::Comment(Box::leak(alices.into_boxed_str())));
    assert_eq!(world.check(&alice, t).await.status(), StatusCode::OK);

    let bobs = world.key(&bob, t).await;
    world.set(t, Profile::Comment(Box::leak(bobs.into_boxed_str())));
    assert_eq!(world.check(&bob, t).await.status(), StatusCode::OK);
    assert_eq!(world.linked(&bob).await, vec![t.to_string()]);
    assert!(world.linked(&alice).await.is_empty());
    assert_eq!(
        world.unlink(&alice, t).await.status(),
        StatusCode::NOT_FOUND
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_user_can_link_only_so_many_trainers() {
    let Some(world) = World::new().await else {
        return;
    };
    let alice = world.sign_in(1).await;
    for n in 1..=MAX_LINKED_TRAINERS as u64 {
        assert_eq!(world.prove(&alice, world.trainer(n)).await, StatusCode::OK);
    }
    assert_eq!(
        world.prove(&alice, world.trainer(9)).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(world.linked(&alice).await.len(), MAX_LINKED_TRAINERS);
    world.clean_up().await;
}

#[tokio::test]
async fn a_refused_link_doesnt_take_the_trainer_from_its_owner() {
    let Some(world) = World::new().await else {
        return;
    };
    let (alice, bob) = (world.sign_in(1).await, world.sign_in(2).await);
    let owned = world.trainer(1);
    assert_eq!(world.prove(&alice, owned).await, StatusCode::OK);
    for n in 2..2 + MAX_LINKED_TRAINERS as u64 {
        assert_eq!(world.prove(&bob, world.trainer(n)).await, StatusCode::OK);
    }
    // Bob has no room left, and the refusal leaves Alice's link alone.
    assert_eq!(world.prove(&bob, owned).await, StatusCode::FORBIDDEN);
    assert_eq!(world.linked(&alice).await, vec![owned.to_string()]);
    world.clean_up().await;
}

#[tokio::test]
async fn unlinking_removes_only_your_own_link() {
    let Some(world) = World::new().await else {
        return;
    };
    let (alice, bob) = (world.sign_in(1).await, world.sign_in(2).await);
    let t = world.trainer(1);
    let key = world.key(&alice, t).await;
    world.set(t, Profile::Comment(Box::leak(key.into_boxed_str())));
    assert_eq!(world.check(&alice, t).await.status(), StatusCode::OK);

    assert_eq!(world.unlink(&bob, t).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(world.linked(&alice).await, vec![t.to_string()]);
    assert_eq!(
        world.unlink(&alice, t).await.status(),
        StatusCode::NO_CONTENT
    );
    assert!(world.linked(&alice).await.is_empty());
    assert_eq!(
        world.unlink(&alice, t).await.status(),
        StatusCode::NOT_FOUND
    );
    world.clean_up().await;
}

#[tokio::test]
async fn too_many_checks_in_an_hour_are_refused_with_a_wait() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    world.key(&session, t).await;
    world.set(t, Profile::NoComment);
    for _ in 0..10 {
        assert_eq!(
            world.check(&session, t).await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let calls = world.lookups.load(Ordering::Relaxed);
    let refused = world.check(&session, t).await;
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    let wait: u64 = refused.headers()[RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=3600).contains(&wait), "{wait}");
    // A refused check never reaches uma.moe.
    assert_eq!(world.lookups.load(Ordering::Relaxed), calls);

    // Another user is unaffected.
    let other = world.sign_in(2).await;
    world.key(&other, t).await;
    assert_eq!(
        world.check(&other, t).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    world.clean_up().await;
}

#[tokio::test]
async fn too_many_new_keys_in_an_hour_are_refused() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    for _ in 0..20 {
        assert_eq!(
            world.start(&session, world.trainer(1)).await.status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        world.start(&session, world.trainer(1)).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    world.clean_up().await;
}

#[tokio::test]
async fn without_trainer_links_the_routes_arent_served() {
    // No Redis needed: the app has no trainer links.
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
    let app = router(Discord::new(config).unwrap());
    let request = Request::builder()
        .uri(TRAINERS_PATH)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
