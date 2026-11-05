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
    ACCOUNT_PATH, CALLBACK_PATH, CHECK_FOR, LINK_PATH, LOGIN_PATH, MAX_LINKED_TRAINERS,
    TRAINERS_PATH, TRAINER_PATH, VERIFICATION_PATH,
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
    links: TrainerLinks,
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
        // Each world has its own queue of running checks, so tests don't take each other's.
        let store = TrainerStore::new(
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
        let links = TrainerLinks::new(store.clone(), lookup);
        let discord = Discord::new(config)
            .unwrap()
            .with_trainer_links(links.clone());
        let world = Self {
            app: router(discord),
            store,
            links,
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
            .map(|t| t["id"].as_str().unwrap().to_owned())
            .collect()
    }

    /// The first pending key in the list, as the page would see it.
    async fn first_pending(&self, session: &str) -> serde_json::Value {
        json_body(self.list(session).await).await["pending"][0].clone()
    }

    async fn discard(&self, session: &str, trainer: u64) -> Response<Body> {
        self.send("DELETE", &Self::uri(VERIFICATION_PATH, trainer), session)
            .await
    }

    async fn clean_up(&self) {
        for n in 1..=3 {
            for t in 1..=12 {
                self.store
                    .unlink(self.user(n), self.trainer(t))
                    .await
                    .unwrap();
                self.store
                    .discard(self.user(n), self.trainer(t))
                    .await
                    .unwrap();
            }
        }
    }
}

fn seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
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
async fn deleting_the_account_removes_its_links_and_keys_and_no_one_elses() {
    let Some(world) = World::new().await else {
        return;
    };
    let (first, second) = (world.sign_in(1).await, world.sign_in(2).await);
    let (mine, pending, theirs) = (world.trainer(1), world.trainer(2), world.trainer(3));
    assert_eq!(world.prove(&first, mine).await, StatusCode::OK);
    let key = world.key(&first, pending).await;
    assert_eq!(world.prove(&second, theirs).await, StatusCode::OK);

    let response = world.send("DELETE", ACCOUNT_PATH, &first).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    assert!(world.linked(&first).await.is_empty());
    assert_eq!(world.linked(&second).await, [theirs.to_string()]);
    assert_eq!(
        world.store.pending(world.user(1), pending).await.unwrap(),
        None
    );
    // The trainer is free: the old key is gone, and a new one is made.
    assert_ne!(world.key(&first, pending).await, key);
    assert_eq!(world.prove(&second, mine).await, StatusCode::OK);
    world.clean_up().await;
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
    assert!(key.starts_with("uma-") && key.len() == 12, "{key}");
    let left = first["expires_in"].as_u64().unwrap();
    assert!((86_000..=86_400).contains(&left), "{left}");

    let again = json_body(world.start(&session, t).await).await;
    assert_eq!(again["key"], first["key"]);
    let other = json_body(world.start(&session, world.trainer(2)).await).await;
    assert_ne!(other["key"], first["key"]);
    world.clean_up().await;
}

#[tokio::test]
async fn a_key_pending_for_one_person_is_never_given_to_another() {
    let Some(world) = World::new().await else {
        return;
    };
    let key = format!("uma-T{}", world.base);
    let (first, second) = (world.user(1), world.user(2));
    let (t1, t2, t3) = (world.trainer(1), world.trainer(2), world.trainer(3));
    let started = world.store.start(first, t1, &key).await.unwrap().unwrap();
    assert_eq!(started.key, key);
    assert_eq!(world.store.start(second, t2, &key).await.unwrap(), None);
    // Same person, other trainer: still taken.
    assert_eq!(world.store.start(first, t3, &key).await.unwrap(), None);
    // Asking again for the same trainer gives the pending key, whatever key is offered.
    let again = world
        .store
        .start(first, t1, "uma-OTHER")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.key, key);

    // Linking uses the key up, so it's free again.
    world
        .store
        .link(first, t1, "Example Trainer")
        .await
        .unwrap();
    let reused = world.store.start(second, t2, &key).await.unwrap().unwrap();
    assert_eq!(reused.key, key);

    // Deleting the account frees its pending keys too.
    world.store.forget(second).await.unwrap();
    assert!(world.store.start(first, t3, &key).await.unwrap().is_some());
    world.store.forget(first).await.unwrap();
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

    // Not there yet: the first look misses, so the check keeps going in the background.
    world.set(t, Profile::Comment("Looking for a club!"));
    let response = world.check(&session, t).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = json_body(response).await;
    assert_eq!(body["checking"], true);
    assert_eq!(body["seconds_left"], CHECK_FOR.as_secs());
    assert!(world.linked(&session).await.is_empty());
    // Pressing Check again joins the running check: no new look at uma.moe.
    let looked = world.lookups.load(Ordering::Relaxed);
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::ACCEPTED
    );
    assert_eq!(world.lookups.load(Ordering::Relaxed), looked);
    // The list shows it, so a page opened again picks it up.
    let pending = world.first_pending(&session).await;
    assert_eq!(pending["trainer"], t.to_string());
    assert_eq!(pending["key"], key);
    assert_eq!(pending["check"]["state"], "checking");

    // Nothing is due before a minute has passed.
    let now = seconds_now();
    assert_eq!(world.links.check_due(now + 30).await.unwrap(), 0);
    // A minute on, the key is there, with other text around it and in any case.
    let comment: &'static str = Box::leak(format!("hi {} :)", key.to_uppercase()).into_boxed_str());
    world.set(t, Profile::Comment(comment));
    assert_eq!(world.links.check_due(now + 61).await.unwrap(), 1);
    assert_eq!(world.linked(&session).await, vec![t.to_string()]);
    let body = json_body(world.list(&session).await).await;
    assert_eq!(body["linked"][0]["name"], "Example Trainer");
    assert_eq!(body["pending"], json!([]));
    world.clean_up().await;
}

#[tokio::test]
async fn the_key_found_on_the_first_look_links_at_once() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    let key = world.key(&session, t).await;
    // With other text around it and in any case.
    let comment: &'static str = Box::leak(format!("hi {} :)", key.to_uppercase()).into_boxed_str());
    world.set(t, Profile::Comment(comment));
    let response = world.check(&session, t).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let body = json_body(response).await;
    assert_eq!(body, json!({ "linked": true, "name": "Example Trainer" }));
    assert_eq!(world.linked(&session).await, vec![t.to_string()]);
    let body = json_body(world.list(&session).await).await;
    assert_eq!(
        body["linked"],
        json!([{ "id": t.to_string(), "name": "Example Trainer" }])
    );

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
async fn a_check_gives_up_after_five_minutes_of_looking_every_minute() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    world.key(&session, t).await;
    world.set(t, Profile::NoComment);
    let now = seconds_now();
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::ACCEPTED
    );
    // A look a minute (and a bit) apart, until five minutes have passed.
    for minute in 1..=5 {
        assert_eq!(world.links.check_due(now + 65 * minute).await.unwrap(), 1);
    }
    assert_eq!(world.lookups.load(Ordering::Relaxed), 6);
    assert_eq!(world.links.check_due(now + 65 * 6).await.unwrap(), 0);
    let pending = world.first_pending(&session).await;
    assert_eq!(pending["check"]["state"], "missing");
    // The key still works: a new check starts.
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::ACCEPTED
    );
    world.clean_up().await;
}

#[tokio::test]
async fn forgetting_a_key_stops_its_check() {
    let Some(world) = World::new().await else {
        return;
    };
    let session = world.sign_in(1).await;
    let t = world.trainer(1);
    world.key(&session, t).await;
    world.set(t, Profile::NoComment);
    let now = seconds_now();
    assert_eq!(
        world.check(&session, t).await.status(),
        StatusCode::ACCEPTED
    );
    assert_eq!(
        world.discard(&session, t).await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(world.first_pending(&session).await, serde_json::Value::Null);
    assert_eq!(world.links.check_due(now + 65).await.unwrap(), 0);
    assert_eq!(
        world.check(&session, t).await.status(),
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
    // An outage is tried again in the background.
    assert_eq!(
        world.check(&session, down).await.status(),
        StatusCode::ACCEPTED
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
    assert_eq!(world.check(&bob, t).await.status(), StatusCode::ACCEPTED);
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
    world.set(t, Profile::NoComment);
    // Each check runs for five minutes, so start over with a new key between them.
    for _ in 0..10 {
        world.key(&session, t).await;
        assert_eq!(
            world.check(&session, t).await.status(),
            StatusCode::ACCEPTED
        );
        world.discard(&session, t).await;
    }
    world.key(&session, t).await;
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
    assert_eq!(world.check(&other, t).await.status(), StatusCode::ACCEPTED);
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
