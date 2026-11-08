//! The board of ads, against a mock Discord and Redis. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, AdStore, Applicant, ApplicantLookup, ApplicationId, ApplicationStore, Applying,
    BotToken, ClientSecret, ClubLookup, ClubStore, ClubSummary, Discord, DiscordConfig,
    DiscordWish, KeySecret, Language, NewAd, OptIn, PublicKey, PublicUrl, SettingsUpdate,
    SigningKey, TrainerLinks, TrainerLookup, TrainerStore, TrainerSummary, ACCOUNT_PATH, ADS_PATH,
    CALLBACK_PATH, LOGIN_PATH, MAX_INVITES, MY_AD_PATH, MY_AD_RENEW_PATH, PAGE_SIZE,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A mock Discord and an app using it and Redis, with IDs no other test uses.
///
/// Club 1 is linked to a server and led by the leader's trainer. The player and the stranger are
/// signed in too; the player has linked a trainer and the stranger hasn't.
struct World {
    mock: MockServer,
    app: Router,
    clubs: ClubStore,
    trainers: TrainerStore,
    ads: AdStore,
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
        let secret = || KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap();
        let clubs = ClubStore::new(conn.clone(), &secret());
        let trainers = TrainerStore::new(conn.clone(), secret())
            .with_check_queue(&format!("test:checks:{base}"));
        let ads = AdStore::new(conn.clone(), secret());
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

        let leader_trainer = base + 50;
        let club = base + 11;
        let lookup = ClubLookup::new(move |id| async move {
            Ok((id == club).then(|| ClubSummary {
                name: "Example Club".into(),
                description: None,
                leader: Some(leader_trainer),
            }))
        });
        let profile = TrainerLookup::new(|_| async {
            Ok(Some(TrainerSummary {
                name: "Example Trainer".into(),
                comment: None,
                club: None,
            }))
        });
        let applicants = ApplicantLookup::new(|trainer| async move {
            Ok(Some(Applicant {
                name: format!("Trainer {trainer}"),
                club_name: Some("Other Club".into()),
                pace_per_day: Some(6_400_000.0),
                fans_this_month: Some(48_000_000),
                alltime: None,
            }))
        });
        let discord = Discord::new(config)
            .unwrap()
            .with_opt_in(OptIn::new(clubs.clone(), lookup))
            .with_trainer_links(TrainerLinks::new(trainers.clone(), profile))
            .with_applications(
                Applying::new(ApplicationStore::new(conn, secret()), applicants)
                    .with_ads(ads.clone()),
            );
        let world = Self {
            mock,
            app: router(discord),
            clubs,
            trainers,
            ads,
            base,
        };
        world.mock_discord().await;
        world
            .clubs
            .link(world.club(), world.server())
            .await
            .unwrap();
        world
            .clubs
            .update(
                world.club(),
                world.server(),
                &SettingsUpdate {
                    leader: Some(world.leader()),
                    ..SettingsUpdate::default()
                },
            )
            .await
            .unwrap();
        for (user, trainer) in [
            (world.leader(), world.base + 50),
            (world.player(), world.base + 51),
        ] {
            world.trainers.link(user, trainer, "Trainer").await.unwrap();
        }
        Some(world)
    }

    fn server(&self) -> u64 {
        self.base + 1
    }
    fn club(&self) -> u64 {
        self.base + 11
    }
    fn leader(&self) -> u64 {
        self.base + 60
    }
    fn player(&self) -> u64 {
        self.base + 61
    }
    fn stranger(&self) -> u64 {
        self.base + 62
    }

    async fn mock_discord(&self) {
        for (code, access) in [
            ("leader-code", "leader-token"),
            ("player-code", "player-token"),
            ("stranger-code", "stranger-token"),
        ] {
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .and(body_string_contains(format!("code={code}")))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json!({ "access_token": access, "token_type": "Bearer" })),
                )
                .mount(&self.mock)
                .await;
        }
        for (access, id) in [
            ("leader-token", self.leader()),
            ("player-token", self.player()),
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
    }

    async fn send(
        &self,
        verb: &str,
        uri: &str,
        cookie: &str,
        body: Option<Value>,
    ) -> Response<Body> {
        let request = Request::builder()
            .method(verb)
            .uri(uri)
            .header(COOKIE, cookie);
        let request = match body {
            Some(body) => request
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        };
        self.app.clone().oneshot(request.unwrap()).await.unwrap()
    }

    async fn sign_in(&self, code: &str) -> String {
        let login = self.send("GET", LOGIN_PATH, "", None).await;
        let location = login.headers()[LOCATION].to_str().unwrap().to_owned();
        let state = query(&location, "state");
        let nonce = cookie(&login, "uma_discord_nonce");
        let uri = format!("{CALLBACK_PATH}?code={code}&state={state}");
        let response = self.send("GET", &uri, &nonce, None).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        cookie(&response, "uma_session")
    }

    fn club_ads(&self) -> String {
        format!("{ADS_PATH}?club={}", self.club())
    }

    fn invite_uri(&self, ad: &str) -> String {
        format!("/api/v1/discord/clubs/{}/ads/{ad}/invite", self.club())
    }

    /// Every ad on the board, page by page, as `cookie` sees it with these query parameters.
    async fn whole_board(&self, cookie: &str, query: &str) -> Vec<Value> {
        let mut found = Vec::new();
        let mut page = 1;
        loop {
            let sep = if query.is_empty() { "" } else { "&" };
            let uri = format!("{ADS_PATH}?{query}{sep}page={page}");
            let response = self.send("GET", &uri, cookie, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            let body = json_body(response).await;
            found.extend(body["ads"].as_array().unwrap().iter().cloned());
            if body["page"].as_u64() >= body["pages"].as_u64() {
                return found;
            }
            page += 1;
        }
    }

    async fn clean_up(&self) {
        self.clubs.unlink(self.club()).await.unwrap();
        for user in [self.leader(), self.player(), self.stranger()] {
            self.ads.delete(user).await.unwrap();
            self.trainers.forget(user).await.unwrap();
        }
    }
}

fn query(location: &str, name: &str) -> String {
    let url = reqwest::Url::parse(location).unwrap();
    let (_, value) = url.query_pairs().find(|(k, _)| k == name).unwrap();
    value.into_owned()
}

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

async fn json_body(response: Response<Body>) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

fn ad() -> Value {
    json!({
        "rankMin": 3,
        "rankMax": 5,
        "discord": "required",
        "language": "en",
    })
}

#[tokio::test]
async fn ads_are_for_people_who_signed_in() {
    let Some(world) = World::new().await else {
        return;
    };
    for (verb, uri) in [
        ("GET", MY_AD_PATH),
        ("PUT", MY_AD_PATH),
        ("DELETE", MY_AD_PATH),
        ("POST", MY_AD_RENEW_PATH),
    ] {
        let response = world.send(verb, uri, "", Some(ad())).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{verb} {uri}");
    }
    let response = world.send("GET", &world.club_ads(), "", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    world.clean_up().await;
}

#[tokio::test]
async fn three_club_ranks_is_the_most_an_ad_can_ask_for() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    for (low, high) in [(3, 5), (4, 4), (9, 11)] {
        let mut body = ad();
        body["rankMin"] = json!(low);
        body["rankMax"] = json!(high);
        let response = world.send("PUT", MY_AD_PATH, &player, Some(body)).await;
        assert_eq!(response.status(), StatusCode::OK, "{low} to {high}");
    }
    world.clean_up().await;
}

#[tokio::test]
async fn a_trainer_posts_an_ad_with_their_fan_numbers() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    let response = world.send("GET", MY_AD_PATH, &player, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, Value::Null);

    let response = world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    assert_eq!(response.status(), StatusCode::OK);
    let posted = json_body(response).await;
    assert_eq!(
        posted["trainerName"],
        format!("Trainer {}", world.base + 51)
    );
    assert_eq!(posted["trainerId"], (world.base + 51).to_string());
    assert_eq!(posted["currentClub"], "Other Club");
    assert_eq!(posted["pace"], 6_400_000.0);
    assert_eq!(posted["fansThisMonth"], 48_000_000);
    assert_eq!(posted["rankMin"], 3);
    assert_eq!(posted["rankMax"], 5);
    assert_eq!(posted["discord"], "required");
    assert_eq!(posted["language"], "en");
    assert_eq!(posted["onBoard"], true);
    assert_eq!(posted["invites"], json!([]));

    let response = world.send("GET", MY_AD_PATH, &player, None).await;
    assert_eq!(json_body(response).await, posted);
    world.clean_up().await;
}

#[tokio::test]
async fn an_ad_needs_a_linked_trainer() {
    let Some(world) = World::new().await else {
        return;
    };
    let stranger = world.sign_in("stranger-code").await;
    let response = world.send("PUT", MY_AD_PATH, &stranger, Some(ad())).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    world.clean_up().await;
}

#[tokio::test]
async fn an_ad_must_make_sense() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    let with = |field: &str, value: Value| {
        let mut body = ad();
        body[field] = value;
        body
    };
    let bad = [
        with("rankMin", json!(0)),
        with("rankMax", json!(12)),
        with("rankMin", json!(9)),
        // More than three ranks: C to B is the widest from C.
        with("rankMax", json!(6)),
        {
            let mut body = ad();
            body["rankMin"] = json!(1);
            body["rankMax"] = json!(11);
            body
        },
        with("discord", json!("maybe")),
        with("language", json!("xx")),
        with("rankMin", json!("high")),
        // Choices that no longer exist are refused, not ignored.
        {
            let mut body = ad();
            body["invitable"] = json!(false);
            body
        },
        {
            let mut body = ad();
            body["showHandle"] = json!(true);
            body
        },
    ];
    for body in bad {
        let response = world
            .send("PUT", MY_AD_PATH, &player, Some(body.clone()))
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
    }
    let response = world.send("GET", MY_AD_PATH, &player, None).await;
    assert_eq!(json_body(response).await, Value::Null);
    world.clean_up().await;
}

#[tokio::test]
async fn anyone_signed_in_sees_the_board_and_only_a_leader_sees_a_clubs_invites() {
    let Some(world) = World::new().await else {
        return;
    };
    let (leader, player) = (
        world.sign_in("leader-code").await,
        world.sign_in("player-code").await,
    );
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;

    // Asking for a club's invites needs its leader.
    let response = world.send("GET", &world.club_ads(), &player, None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // The ads themselves don't.
    let board = world.whole_board(&player, "").await;
    assert!(board.iter().any(
        |ad| ad["trainerId"] == (world.base + 51).to_string().as_str() && ad["invited"] == false
    ));
    let response = world.send("GET", ADS_PATH, "", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let board = world
        .whole_board(&leader, &format!("club={}", world.club()))
        .await;
    let found = board
        .iter()
        .find(|ad| ad["trainerId"] == (world.base + 51).to_string().as_str())
        .expect("the ad is on the board");
    assert!(found.get("discordName").is_none());
    assert_eq!(found["invited"], false);
    assert_eq!(found["id"].as_str().unwrap().len(), 32);

    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_invites_a_trainer_who_sees_and_dismisses_it() {
    let Some(world) = World::new().await else {
        return;
    };
    let (leader, player) = (
        world.sign_in("leader-code").await,
        world.sign_in("player-code").await,
    );
    let posted = json_body(world.send("PUT", MY_AD_PATH, &player, Some(ad())).await).await;
    let id = world.ads.get(world.player()).await.unwrap().unwrap().id;
    assert_eq!(posted["invites"], json!([]));

    // Only the club's leader can invite, and only from a real ad.
    let uri = world.invite_uri(&id);
    let response = world.send("POST", &uri, &player, None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = world
        .send("POST", &world.invite_uri("not-an-ad"), &leader, None)
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = world
        .send(
            "POST",
            &world.invite_uri("00000000000000000000000000000000"),
            &leader,
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    for _ in 0..2 {
        let response = world.send("POST", &uri, &leader, None).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    let board = world
        .whole_board(&leader, &format!("club={}", world.club()))
        .await;
    let found = board.iter().find(|ad| ad["id"] == id).unwrap();
    assert_eq!(found["invited"], true);

    let mine = json_body(world.send("GET", MY_AD_PATH, &player, None).await).await;
    let invites = mine["invites"].as_array().unwrap();
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0]["clubId"], world.club().to_string());
    assert_eq!(invites[0]["clubName"], "Example Club");

    let dismiss = format!("/api/v1/discord/ads/me/invites/{}", world.club());
    let response = world.send("DELETE", &dismiss, &player, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = world.send("DELETE", &dismiss, &player, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    world.clean_up().await;
}

#[tokio::test]
async fn a_club_that_doesnt_take_applications_cant_invite() {
    let Some(world) = World::new().await else {
        return;
    };
    let (leader, player) = (
        world.sign_in("leader-code").await,
        world.sign_in("player-code").await,
    );
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    let id = world.ads.get(world.player()).await.unwrap().unwrap().id;
    world
        .clubs
        .update(
            world.club(),
            world.server(),
            &SettingsUpdate {
                open: Some(false),
                ..SettingsUpdate::default()
            },
        )
        .await
        .unwrap();
    let response = world
        .send("POST", &world.invite_uri(&id), &leader, None)
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(world
        .ads
        .get(world.player())
        .await
        .unwrap()
        .unwrap()
        .invites
        .is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn an_ad_takes_a_limited_number_of_invites() {
    let Some(world) = World::new().await else {
        return;
    };
    let (leader, player) = (
        world.sign_in("leader-code").await,
        world.sign_in("player-code").await,
    );
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    let id = world.ads.get(world.player()).await.unwrap().unwrap().id;
    for club in 1..=MAX_INVITES as u64 {
        world
            .ads
            .invite(&id, world.base + 500 + club)
            .await
            .unwrap();
    }
    let response = world
        .send("POST", &world.invite_uri(&id), &leader, None)
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    world.clean_up().await;
}

#[tokio::test]
async fn renewing_keeps_the_ads_choices_and_its_invites() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    let response = world.send("POST", MY_AD_RENEW_PATH, &player, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    let id = world.ads.get(world.player()).await.unwrap().unwrap().id;
    world.ads.invite(&id, world.club()).await.unwrap();
    let response = world.send("POST", MY_AD_RENEW_PATH, &player, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let renewed = json_body(response).await;
    assert_eq!(renewed["rankMax"], 5);
    assert_eq!(renewed["discord"], "required");
    assert_eq!(renewed["invites"].as_array().unwrap().len(), 1);
    world.clean_up().await;
}

#[tokio::test]
async fn deleting_an_ad_takes_it_off_the_board() {
    let Some(world) = World::new().await else {
        return;
    };
    let (leader, player) = (
        world.sign_in("leader-code").await,
        world.sign_in("player-code").await,
    );
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    let response = world.send("DELETE", MY_AD_PATH, &player, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = world.send("GET", MY_AD_PATH, &player, None).await;
    assert_eq!(json_body(response).await, Value::Null);
    let board = world
        .whole_board(&leader, &format!("club={}", world.club()))
        .await;
    assert!(board
        .iter()
        .all(|ad| ad["trainerId"] != (world.base + 51).to_string().as_str()));
    world.clean_up().await;
}

#[tokio::test]
async fn unlinking_the_trainer_or_deleting_the_account_removes_the_ad() {
    let Some(world) = World::new().await else {
        return;
    };
    // The unlink route only takes trainer IDs of up to 12 digits, which the test IDs can exceed.
    let trainer = 100_000_000 + (world.base / 100) % 800_000_000;
    world.trainers.forget(world.player()).await.unwrap();
    world
        .trainers
        .link(world.player(), trainer, "Trainer")
        .await
        .unwrap();
    let player = world.sign_in("player-code").await;
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    assert!(world.ads.get(world.player()).await.unwrap().is_some());
    let unlink = format!("/api/v1/discord/trainers/{trainer}");
    let response = world.send("DELETE", &unlink, &player, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(world.ads.get(world.player()).await.unwrap(), None);

    world
        .trainers
        .link(world.player(), trainer, "Trainer")
        .await
        .unwrap();
    world.send("PUT", MY_AD_PATH, &player, Some(ad())).await;
    assert!(world.ads.get(world.player()).await.unwrap().is_some());
    let response = world.send("DELETE", ACCOUNT_PATH, &player, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(world.ads.get(world.player()).await.unwrap(), None);
    world.clean_up().await;
}

#[tokio::test]
async fn an_ad_can_ask_for_any_of_the_languages() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    for language in [
        "en", "ja", "ko", "zh", "es", "pt", "fr", "de", "id", "th", "vi", "any",
    ] {
        let mut body = ad();
        body["language"] = json!(language);
        let response = world.send("PUT", MY_AD_PATH, &player, Some(body)).await;
        assert_eq!(response.status(), StatusCode::OK, "{language}");
        assert_eq!(json_body(response).await["language"], language);
    }
    world.clean_up().await;
}

#[tokio::test]
async fn the_board_takes_filters_and_pages() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    // Ads only this test makes: rank SS only, in Vietnamese, needing a Discord server.
    let users: Vec<u64> = (0..23).map(|n| world.base + 200 + n).collect();
    for (n, user) in users.iter().enumerate() {
        world
            .ads
            .save(&NewAd {
                user: *user,
                trainer: 100_000_100 + n as u64,
                trainer_name: format!("Paged {n}"),
                current_club: None,
                pace: Some(9_000_000.0 - n as f64),
                fans_this_month: None,
                rank_min: 11,
                rank_max: 11,
                discord: DiscordWish::Required,
                language: Language::Vietnamese,
            })
            .await
            .unwrap();
    }
    let ours = |body: &Value| -> Vec<String> {
        body["ads"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|ad| ad["trainerName"].as_str().unwrap().starts_with("Paged "))
            .map(|ad| ad["trainerName"].as_str().unwrap().to_owned())
            .collect()
    };
    let query = "rank_min=11&rank_max=11&language=vi";
    let first = json_body(
        world
            .send("GET", &format!("{ADS_PATH}?{query}"), &player, None)
            .await,
    )
    .await;
    assert_eq!(first["page"], 1);
    assert!(first["pages"].as_u64().unwrap() >= 2);
    assert!(first["total"].as_u64().unwrap() >= 23);
    assert_eq!(first["ads"].as_array().unwrap().len(), PAGE_SIZE);
    // Fastest first.
    assert_eq!(ours(&first)[0], "Paged 0");

    let whole = world.whole_board(&player, query).await;
    let named: Vec<&str> = whole
        .iter()
        .filter_map(|ad| ad["trainerName"].as_str())
        .filter(|name| name.starts_with("Paged "))
        .collect();
    assert_eq!(named.len(), 23);

    // A club without a Discord server, or one of another rank or language, sees none of them.
    for other in [
        "rank_min=11&rank_max=11&language=vi&has_discord=false",
        "rank_min=1&rank_max=10&language=vi",
        "rank_min=11&rank_max=11&language=ja",
    ] {
        let page = world.whole_board(&player, other).await;
        assert!(
            page.iter()
                .all(|ad| !ad["trainerName"].as_str().unwrap().starts_with("Paged ")),
            "{other}"
        );
    }

    for bad in ["page=0", "rank_min=12", "has_discord=maybe", "language=xx"] {
        let response = world
            .send("GET", &format!("{ADS_PATH}?{bad}"), &player, None)
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{bad}");
    }
    for user in users {
        world.ads.delete(user).await.unwrap();
    }
    world.clean_up().await;
}
