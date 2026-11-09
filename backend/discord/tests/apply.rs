//! Applying to a club, against a mock Discord and Redis. All values are made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Body;
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use ed25519_dalek::{Signer, SigningKey as Ed25519Key};
use http_body_util::BodyExt;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_discord::{
    router, Applicant, ApplicantLookup, ApplicationId, ApplicationStore, Applying, BotToken,
    ClientSecret, ClubLookup, ClubStore, ClubSummary, Discord, DiscordConfig, KeySecret, OptIn,
    PublicKey, PublicUrl, SettingsUpdate, SigningKey, TicketName, TrainerLinks, TrainerLookup,
    TrainerStore, TrainerSummary, APPLICATIONS_PATH, APPLY_PATH, CALLBACK_PATH,
    CLUB_APPLICATIONS_PATH, INTERACTIONS_PATH, LOGIN_PATH, MAX_PER_DAY,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A mock Discord and an app using it and Redis, with IDs no other test uses.
///
/// Club 1 is linked to a server. Its leader's trainer is one the leader user linked, and the
/// leader user is the one noted to get its tickets. The player has linked a trainer too.
struct World {
    mock: MockServer,
    app: Router,
    clubs: ClubStore,
    trainers: TrainerStore,
    applications: ApplicationStore,
    base: u64,
}

fn key() -> Ed25519Key {
    Ed25519Key::from_bytes(&[7; 32])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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
        let applications = ApplicationStore::new(conn, secret());
        let mock = MockServer::start().await;
        let mut config = DiscordConfig::new(
            ApplicationId::parse("1000").unwrap(),
            PublicKey::parse(&hex(key().verifying_key().as_bytes())).unwrap(),
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
                leader_chara_dress_id: None,
                alltime: None,
            }))
        });
        let discord = Discord::new(config)
            .unwrap()
            .with_opt_in(OptIn::new(clubs.clone(), lookup))
            .with_trainer_links(TrainerLinks::new(trainers.clone(), profile))
            .with_applications(Applying::new(applications.clone(), applicants));
        let world = Self {
            mock,
            app: router(discord),
            clubs,
            trainers,
            applications,
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
    fn channel(&self) -> u64 {
        self.base + 70
    }
    fn message(&self) -> u64 {
        self.base + 71
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
        Mock::given(method("GET"))
            .and(path(format!("/users/{}", self.player())))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "username": "example_player", "global_name": null })),
            )
            .mount(&self.mock)
            .await;
    }

    /// Lets the bot add the player, open the channel and post in it.
    async fn mock_ticket(&self) {
        Mock::given(method("PUT"))
            .and(path(format!(
                "/guilds/{}/members/{}",
                self.server(),
                self.player()
            )))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({})))
            .mount(&self.mock)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/guilds/{}/channels", self.server())))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({ "id": self.channel().to_string() })),
            )
            .mount(&self.mock)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/channels/{}/messages", self.channel())))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "id": self.message().to_string() })),
            )
            .mount(&self.mock)
            .await;
    }

    async fn get(&self, uri: &str, cookie: &str) -> Response<Body> {
        let request = Request::get(uri).header(COOKIE, cookie);
        self.app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn sign_in(&self, code: &str) -> String {
        let login = self.get(LOGIN_PATH, "").await;
        let state = query(&location(&login), "state");
        let nonce = cookie(&login, "uma_discord_nonce");
        let uri = format!("{CALLBACK_PATH}?code={code}&state={state}");
        let response = self.get(&uri, &nonce).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        cookie(&response, "uma_session")
    }

    async fn start(&self, session: &str, club: &str) -> Response<Body> {
        self.get(&format!("{APPLY_PATH}?club={club}"), session)
            .await
    }

    /// Finishes an application the way Discord's redirect would.
    async fn finish(&self, started: &Response<Body>, code: &str) -> Response<Body> {
        let state = query(&location(started), "state");
        let nonce = cookie(started, "uma_discord_nonce");
        self.get(
            &format!("{CALLBACK_PATH}?code={code}&state={state}"),
            &nonce,
        )
        .await
    }

    /// Applies as the player, start to finish.
    async fn apply(&self, session: &str) -> Response<Body> {
        let started = self.start(session, &self.club().to_string()).await;
        assert_eq!(started.status(), StatusCode::TEMPORARY_REDIRECT);
        self.finish(&started, "player-code").await
    }

    async fn click(&self, custom_id: &str, user: u64, permissions: &str) -> Value {
        let body = json!({
            "type": 3,
            "data": { "custom_id": custom_id },
            "member": { "user": { "id": user.to_string() }, "permissions": permissions },
        })
        .to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let signature = key().sign(format!("{timestamp}{body}").as_bytes());
        let request = Request::post(INTERACTIONS_PATH)
            .header("x-signature-ed25519", hex(&signature.to_bytes()))
            .header("x-signature-timestamp", timestamp)
            .body(Body::from(body))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        json_body(response).await
    }

    async fn requests(&self, verb: &str, suffix: &str) -> Vec<Value> {
        self.mock
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.method.as_str() == verb && r.url.path().ends_with(suffix))
            .map(|r| serde_json::from_slice(&r.body).unwrap_or(Value::Null))
            .collect()
    }

    async fn clean_up(&self) {
        self.clubs.unlink(self.club()).await.unwrap();
        for user in [self.leader(), self.player(), self.stranger()] {
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

#[tokio::test]
async fn starting_without_a_session_signs_the_player_in_and_comes_back() {
    let Some(world) = World::new().await else {
        return;
    };
    let response = world.start("", &world.club().to_string()).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let url = location(&response);
    assert_eq!(query(&url, "scope"), "identify");
    world.clean_up().await;
}

#[tokio::test]
async fn starting_needs_a_club_that_takes_applications_and_a_linked_trainer() {
    let Some(world) = World::new().await else {
        return;
    };
    let player = world.sign_in("player-code").await;
    let stranger = world.sign_in("stranger-code").await;
    let status = |response: Response<Body>| response.status();

    assert_eq!(
        status(world.start(&player, "abc").await),
        StatusCode::BAD_REQUEST
    );
    // A club nobody linked.
    assert_eq!(
        status(world.start(&player, &(world.base + 99).to_string()).await),
        StatusCode::NOT_FOUND
    );
    // No trainer linked.
    assert_eq!(
        status(world.start(&stranger, &world.club().to_string()).await),
        StatusCode::FORBIDDEN
    );
    // Closed to applications.
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
    assert_eq!(
        status(world.start(&player, &world.club().to_string()).await),
        StatusCode::NOT_FOUND
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_browser_is_sent_to_the_applications_page_to_see_what_went_wrong() {
    let Some(world) = World::new().await else {
        return;
    };
    let stranger = world.sign_in("stranger-code").await;
    let request = Request::get(format!("{APPLY_PATH}?club={}", world.club()))
        .header(COOKIE, stranger)
        .header("accept", "text/html")
        .body(Body::empty())
        .unwrap();
    let response = world.app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        location(&response),
        "/applications?error=no_trainer".to_owned()
    );
    world.clean_up().await;
}

/// That `channel` is named `stem`, then the rest of the name, then a dash and four digits.
fn assert_ticket_name(channel: &Value, stem: &str) {
    let name = channel["name"].as_str().unwrap();
    let rest = name.strip_prefix(stem).unwrap_or_else(|| panic!("{name}"));
    let (_, number) = rest.rsplit_once('-').unwrap_or_else(|| panic!("{name}"));
    assert!(
        number.len() == 4 && number.bytes().all(|b| b.is_ascii_digit()),
        "{name}"
    );
}

#[tokio::test]
async fn tickets_are_named_as_the_leader_chose() {
    for (name, stem) in [
        (
            TicketName::with_prefix("Apply Here").unwrap(),
            "apply-here-trainer",
        ),
        (TicketName::with_override("Join Us").unwrap(), "join-us"),
    ] {
        let Some(world) = World::new().await else {
            return;
        };
        let update = SettingsUpdate {
            ticket_name: Some(name),
            ..SettingsUpdate::default()
        };
        world
            .clubs
            .update(world.club(), world.server(), &update)
            .await
            .unwrap();
        world.mock_ticket().await;
        let player = world.sign_in("player-code").await;
        let started = world.start(&player, &world.club().to_string()).await;
        world.finish(&started, "player-code").await;

        let channels = world.requests("POST", "/channels").await;
        assert_eq!(channels.len(), 1);
        assert_ticket_name(&channels[0], stem);
        world.clean_up().await;
    }
}

#[tokio::test]
async fn applying_joins_the_player_and_opens_a_ticket_for_them_and_the_leader() {
    let Some(world) = World::new().await else {
        return;
    };
    world.mock_ticket().await;
    let player = world.sign_in("player-code").await;
    let started = world.start(&player, &world.club().to_string()).await;
    let url = location(&started);
    assert_eq!(query(&url, "scope"), "identify guilds.join");

    let response = world.finish(&started, "player-code").await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(location(&response), "/applications/trainer");

    let joins = world
        .requests("PUT", &format!("/members/{}", world.player()))
        .await;
    assert_eq!(joins, vec![json!({ "access_token": "player-token" })]);

    let channels = world.requests("POST", "/channels").await;
    assert_eq!(channels.len(), 1);
    let overwrites = channels[0]["permission_overwrites"].as_array().unwrap();
    let ids: Vec<&str> = overwrites
        .iter()
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            world.server().to_string(),
            "1000".to_owned(),
            world.player().to_string(),
            world.leader().to_string(),
        ]
    );
    // Everyone else is shut out of it.
    assert_eq!(overwrites[0]["deny"], (1u64 << 10).to_string());
    // Named after the club and the trainer, and a number.
    assert_ticket_name(&channels[0], "example-club-trainer");

    let messages = world.requests("POST", "/messages").await;
    assert_eq!(messages.len(), 1);
    let fields = messages[0]["embeds"][0]["fields"].to_string();
    assert!(fields.contains("https://uma.moe/profile/"), "{fields}");
    assert!(fields.contains("6,400,000/day"));

    // The player sees it, with the ticket's link.
    let mine = json_body(world.get(APPLICATIONS_PATH, &player).await).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["clubId"], world.club().to_string());
    assert_eq!(mine[0]["status"], "pending");
    assert_eq!(
        mine[0]["ticketUrl"],
        format!(
            "https://discord.com/channels/{}/{}",
            world.server(),
            world.channel()
        )
    );

    // So does the club's leader.
    let leader = world.sign_in("leader-code").await;
    let uri = CLUB_APPLICATIONS_PATH.replace("{club}", &world.club().to_string());
    let listed = json_body(world.get(&uri, &leader).await).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["discordName"], "example_player");
    assert_eq!(listed[0]["currentClub"], "Other Club");
    assert_eq!(listed[0]["fansThisMonth"], 48_000_000);
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_clubs_leader_sees_its_applications() {
    let Some(world) = World::new().await else {
        return;
    };
    let uri = CLUB_APPLICATIONS_PATH.replace("{club}", &world.club().to_string());
    assert_eq!(world.get(&uri, "").await.status(), StatusCode::UNAUTHORIZED);
    let player = world.sign_in("player-code").await;
    assert_eq!(
        world.get(&uri, &player).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        world.get(APPLICATIONS_PATH, "").await.status(),
        StatusCode::UNAUTHORIZED
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_second_application_to_the_same_club_waits_for_an_answer() {
    let Some(world) = World::new().await else {
        return;
    };
    world.mock_ticket().await;
    let player = world.sign_in("player-code").await;
    assert_eq!(
        world.apply(&player).await.status(),
        StatusCode::TEMPORARY_REDIRECT
    );

    let again = world.apply(&player).await;
    assert_eq!(again.status(), StatusCode::CONFLICT);
    assert_eq!(world.requests("POST", "/channels").await.len(), 1);

    let id = world
        .applications
        .for_applicant(world.player())
        .await
        .unwrap()[0]
        .id
        .clone();
    let answer = world
        .click(&format!("apply:decline:{id}"), world.leader(), "0")
        .await;
    assert_eq!(answer["type"], 7);
    assert_eq!(
        world.apply(&player).await.status(),
        StatusCode::TEMPORARY_REDIRECT
    );
    world.clean_up().await;
}

#[tokio::test]
async fn a_player_can_only_send_so_many_applications_a_day() {
    let Some(world) = World::new().await else {
        return;
    };
    world.mock_ticket().await;
    let player = world.sign_in("player-code").await;
    // Applications to other clubs fill the day's limit.
    for n in 0..u64::from(MAX_PER_DAY) {
        let new = uma_stable_match_discord::NewApplication {
            club: world.base + 80 + n,
            club_name: "Elsewhere".into(),
            guild: 1,
            applicant: world.player(),
            trainer: world.base + 51,
            trainer_name: "Trainer".into(),
            current_club: None,
            pace: None,
            fans_this_month: None,
        };
        world.applications.create(&new).await.unwrap();
    }
    let response = world.apply(&player).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(world.requests("POST", "/channels").await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn nothing_is_kept_when_the_ticket_cant_be_opened() {
    let Some(world) = World::new().await else {
        return;
    };
    Mock::given(method("PUT"))
        .and(path(format!(
            "/guilds/{}/members/{}",
            world.server(),
            world.player()
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&world.mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/guilds/{}/channels", world.server())))
        .respond_with(ResponseTemplate::new(500))
        .mount(&world.mock)
        .await;
    let player = world.sign_in("player-code").await;
    let response = world.apply(&player).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(world
        .applications
        .for_applicant(world.player())
        .await
        .unwrap()
        .is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_person_who_started_can_finish() {
    let Some(world) = World::new().await else {
        return;
    };
    world.mock_ticket().await;
    let player = world.sign_in("player-code").await;
    let started = world.start(&player, &world.club().to_string()).await;
    let response = world.finish(&started, "stranger-code").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(world.requests("POST", "/channels").await.is_empty());
    world.clean_up().await;
}

#[tokio::test]
async fn only_the_leader_answers_with_the_buttons() {
    let Some(world) = World::new().await else {
        return;
    };
    world.mock_ticket().await;
    let player = world.sign_in("player-code").await;
    assert_eq!(
        world.apply(&player).await.status(),
        StatusCode::TEMPORARY_REDIRECT
    );
    let id = world
        .applications
        .for_applicant(world.player())
        .await
        .unwrap()[0]
        .id
        .clone();
    let accept = format!("apply:accept:{id}");

    // The applicant can't answer their own application, nor can a server manager when the club
    // has a leader.
    for (user, permissions) in [(world.player(), "0"), (world.stranger(), "32")] {
        let reply = world.click(&accept, user, permissions).await;
        assert_eq!(reply["type"], 4, "{reply}");
        assert_eq!(reply["data"]["flags"], 64);
    }
    let pending = world.applications.get(&id).await.unwrap().unwrap();
    assert_eq!(pending.status, uma_stable_match_discord::Status::Pending);

    let reply = world.click(&accept, world.leader(), "0").await;
    assert_eq!(reply["type"], 7);
    assert_eq!(reply["data"]["components"], json!([]));
    let answered = world.applications.get(&id).await.unwrap().unwrap();
    assert_eq!(answered.status, uma_stable_match_discord::Status::Accepted);

    // A second click finds it answered.
    let reply = world
        .click(&format!("apply:decline:{id}"), world.leader(), "0")
        .await;
    assert_eq!(reply["type"], 4);
    assert!(reply["data"]["content"]
        .as_str()
        .unwrap()
        .contains("already"));
    world.clean_up().await;
}

#[tokio::test]
async fn a_leader_who_left_the_server_doesnt_stop_the_ticket() {
    let Some(world) = World::new().await else {
        return;
    };
    Mock::given(method("PUT"))
        .and(path(format!(
            "/guilds/{}/members/{}",
            world.server(),
            world.player()
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&world.mock)
        .await;
    // Discord refuses the channel while the leader is in its overwrites.
    Mock::given(method("POST"))
        .and(path(format!("/guilds/{}/channels", world.server())))
        .and(body_string_contains(world.leader().to_string()))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({ "message": "Unknown Member", "code": 10007 })),
        )
        .mount(&world.mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/guilds/{}/channels", world.server())))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({ "id": world.channel().to_string() })),
        )
        .mount(&world.mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/channels/{}/messages", world.channel())))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "id": world.message().to_string() })),
        )
        .mount(&world.mock)
        .await;
    let player = world.sign_in("player-code").await;
    assert_eq!(
        world.apply(&player).await.status(),
        StatusCode::TEMPORARY_REDIRECT
    );
    world.clean_up().await;
}
