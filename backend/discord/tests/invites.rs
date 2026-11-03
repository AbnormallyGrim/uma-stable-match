//! Invite resolution tests against a mock Discord. All values are made up.

use serde_json::json;
use uma_stable_match_discord::{
    resolve_invite, servers_in_description, ApplicationId, BotToken, ClientSecret, DiscordConfig,
    PublicKey, PublicUrl, Rest, SigningKey,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn rest(server: &MockServer) -> Rest {
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
    Rest::new(&config).unwrap()
}

async fn invite_to(server: &MockServer, code: &str, guild_id: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/invites/{code}")))
        .and(header("authorization", "Bot made-up-bot-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "code": code, "guild": { "id": guild_id, "name": "Made Up Server" } }),
        ))
        .mount(server)
        .await;
}

async fn unknown_invite(server: &MockServer, code: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/invites/{code}")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({ "message": "Unknown Invite", "code": 10006 })),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn a_working_invite_gives_its_server() {
    let server = MockServer::start().await;
    invite_to(&server, "abc123", "5000").await;
    assert_eq!(
        resolve_invite(&rest(&server), "abc123").await.unwrap(),
        Some(5000)
    );
}

#[tokio::test]
async fn an_expired_invite_gives_no_server() {
    let server = MockServer::start().await;
    unknown_invite(&server, "old123").await;
    assert_eq!(
        resolve_invite(&rest(&server), "old123").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn an_invite_that_isnt_for_a_server_gives_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/invites/dm1234"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": "dm1234" })))
        .mount(&server)
        .await;
    assert_eq!(
        resolve_invite(&rest(&server), "dm1234").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn a_discord_outage_is_an_error_not_a_missing_invite() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let error = resolve_invite(&rest(&server), "abc123").await.unwrap_err();
    assert_eq!(error.status().map(|s| s.as_u16()), Some(503));
}

#[tokio::test]
async fn several_invites_to_one_server_give_it_once() {
    let server = MockServer::start().await;
    invite_to(&server, "aaa111", "5000").await;
    invite_to(&server, "bbb222", "5000").await;
    unknown_invite(&server, "old123").await;
    let text = "discord.gg/aaa111, discord.gg/old123 and discord.com/invite/bbb222";
    assert_eq!(
        servers_in_description(&rest(&server), text).await.unwrap(),
        [5000]
    );
}

#[tokio::test]
async fn invites_to_different_servers_give_each() {
    let server = MockServer::start().await;
    invite_to(&server, "aaa111", "5000").await;
    invite_to(&server, "bbb222", "6000").await;
    let text = "discord.gg/bbb222 discord.gg/aaa111";
    assert_eq!(
        servers_in_description(&rest(&server), text).await.unwrap(),
        [6000, 5000]
    );
}

#[tokio::test]
async fn no_invite_or_only_dead_ones_give_no_server() {
    let server = MockServer::start().await;
    unknown_invite(&server, "old123").await;
    let rest = rest(&server);
    assert!(servers_in_description(&rest, "no link here")
        .await
        .unwrap()
        .is_empty());
    assert!(servers_in_description(&rest, "discord.gg/old123")
        .await
        .unwrap()
        .is_empty());
    // The first needed no request at all.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn an_outage_part_way_fails_instead_of_guessing() {
    let server = MockServer::start().await;
    invite_to(&server, "aaa111", "5000").await;
    Mock::given(method("GET"))
        .and(path("/invites/bbb222"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let text = "discord.gg/aaa111 discord.gg/bbb222";
    assert!(servers_in_description(&rest(&server), text).await.is_err());
}
