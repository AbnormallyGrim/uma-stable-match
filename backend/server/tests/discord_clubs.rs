//! Discord's club lookup against a mock server. All data is made up.

use std::time::Duration;

use serde_json::json;
use uma_stable_match_server::{club_lookup, ClubDetails, DetailConfig, SharedPool};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn details(server: &MockServer) -> ClubDetails {
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
    ClubDetails::new(client, SharedPool::default(), DetailConfig::default()).0
}

#[tokio::test]
async fn gives_a_clubs_description() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "900001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "circle": {
                "circle_id": 900001,
                "name": "Made-up Club",
                "member_count": 9,
                "comment": "Join discord.gg/example",
            },
            "members": [],
        })))
        .mount(&server)
        .await;
    let lookup = club_lookup(details(&server));
    let club = lookup.get(900001).await.unwrap().unwrap();
    assert_eq!(club.description.as_deref(), Some("Join discord.gg/example"));
}

#[tokio::test]
async fn a_failing_source_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let lookup = club_lookup(details(&server));
    assert!(lookup.get(900002).await.is_err());
}
