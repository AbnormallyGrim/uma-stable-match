//! Trainer lookup against a mock server. All data is made up.

use std::time::Duration;

use serde_json::json;
use uma_stable_match_server::{trainer_lookup, Source};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, RetryConfig};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn source(server: &MockServer) -> Source {
    Source::from(
        Client::new(ClientConfig {
            base_url: server.uri(),
            min_interval: Duration::ZERO,
            retry: RetryConfig {
                max_retries: 0,
                ..Default::default()
            },
            ..ClientConfig::new(ApiKey::new("test-key"))
        })
        .unwrap(),
    )
}

#[tokio::test]
async fn gives_a_trainers_name_and_comment() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/user/profile/100000001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "trainer": {
                "account_id": "100000001",
                "name": "Example Trainer",
                "comment": "Hello uma-7KQ4XM",
            },
        })))
        .expect(1)
        .mount(&server)
        .await;
    let trainer = trainer_lookup(source(&server)).get(100000001).await;
    let trainer = trainer.unwrap().unwrap();
    assert_eq!(trainer.name, "Example Trainer");
    assert_eq!(trainer.comment.as_deref(), Some("Hello uma-7KQ4XM"));
}

#[tokio::test]
async fn a_trainer_without_a_comment_has_none() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/user/profile/100000001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "trainer": { "account_id": "100000001", "name": "Example Trainer", "comment": null },
        })))
        .mount(&server)
        .await;
    let trainer = trainer_lookup(source(&server)).get(100000001).await;
    assert_eq!(trainer.unwrap().unwrap().comment, None);
}

#[tokio::test]
async fn an_unknown_trainer_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/user/profile/100000002"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let trainer = trainer_lookup(source(&server)).get(100000002).await;
    assert_eq!(trainer.unwrap(), None);
}

#[tokio::test]
async fn a_failing_source_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/user/profile/100000003"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    assert!(trainer_lookup(source(&server))
        .get(100000003)
        .await
        .is_err());
}
