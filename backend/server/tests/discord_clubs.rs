//! Discord's club lookup against a mock server. All data is made up.

use std::time::Duration;

use serde_json::json;
use uma_stable_match_server::{club_lookup, Source};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn source(server: &MockServer) -> Source {
    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    Source::from(client)
}

#[tokio::test]
async fn gives_a_clubs_name_description_and_leader() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": {
                "circle_id": 900001,
                "name": "Made-up Club",
                "member_count": 9,
                "comment": "Join discord.gg/example",
                "leader_viewer_id": 100000001,
            },
            "members": [],
        })))
        .mount(&server)
        .await;
    let lookup = club_lookup(source(&server));
    let club = lookup.get(900001).await.unwrap().unwrap();
    assert_eq!(club.name, "Made-up Club");
    assert_eq!(club.description.as_deref(), Some("Join discord.gg/example"));
    assert_eq!(club.leader, Some(100000001));
}

#[tokio::test]
async fn an_unknown_club_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let lookup = club_lookup(source(&server));
    assert_eq!(lookup.get(900003).await.unwrap(), None);
}

#[tokio::test]
async fn a_failing_source_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let lookup = club_lookup(source(&server));
    assert!(lookup.get(900002).await.is_err());
}
