use uma_stable_match_umamoe_client::{ApiKey, ClientConfig, StatusCode};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn details(queue_size: usize) -> (ClubDetails, BackgroundLoader) {
    let config = ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    };
    let client = Client::new(config).unwrap();
    let detail_config = DetailConfig {
        queue_size,
        ..DetailConfig::default()
    };
    ClubDetails::new(&client, SharedPool::default(), detail_config)
}

#[tokio::test]
async fn queue_skips_duplicates() {
    let (details, _loader) = details(10);
    assert!(details.queue(1));
    assert!(!details.queue(1));
    assert!(details.queue(2));
    assert_eq!(details.queued(), 2);
}

#[tokio::test]
async fn queue_is_bounded() {
    let (details, _loader) = details(2);
    assert!(details.queue(1));
    assert!(details.queue(2));
    assert!(!details.queue(3));
    assert_eq!(details.queued(), 2);
}

#[test]
fn default_config() {
    let config = DetailConfig::default();
    assert_eq!(config.refresh, RefreshConfig::default());
    assert!(!config.refresher);
    assert!(config.max_clubs > 0);
    assert!(config.queue_size > 0);
}

#[tokio::test]
async fn visitor_fetches_give_up_on_a_long_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    let (details, _loader) =
        ClubDetails::new(&client, SharedPool::default(), DetailConfig::default());

    let started = Instant::now();
    let error = details
        .get(900_000_101, Priority::Interactive)
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::TOO_MANY_REQUESTS));
    assert!(started.elapsed() < Duration::from_secs(5));
}
