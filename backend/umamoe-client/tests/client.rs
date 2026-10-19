//! Public API tests against a mock server. All data is made up.

use std::time::Duration;

use uma_stable_match_scoring::{Club, JoinStyle};
use uma_stable_match_umamoe_client::{
    ApiKey, CircleRequest, CircleSort, Client, ClientConfig, Error, ListCirclesRequest, SortDir,
    StatusCode,
};
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "test-key";
const CIRCLE: &str = include_str!("fixtures/circle.json");
const CIRCLE_LIST: &str = include_str!("fixtures/circle_list.json");

fn config(server: &MockServer) -> ClientConfig {
    ClientConfig {
        base_url: server.uri(),
        timeout: Duration::from_secs(5),
        ..ClientConfig::new(ApiKey::new(KEY))
    }
}

fn client(server: &MockServer) -> Client {
    Client::new(config(server)).unwrap()
}

fn json(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body, "application/json")
}

#[tokio::test]
async fn fetches_circle_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/circles"))
        .and(query_param("circle_id", "900001"))
        .and(query_param_is_missing("viewer_id"))
        .and(header("x-api-key", KEY))
        .and(header("accept", "application/json"))
        .respond_with(json(CIRCLE))
        .expect(1)
        .mount(&server)
        .await;

    let response = client(&server)
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap();
    assert_eq!(response.circle.name, "Example Club");
    assert_eq!(response.members.len(), 3);
    assert_eq!(response.members[0].daily_fans.len(), 31);
    assert_eq!(response.club_rank, Some(6));

    let club = Club::from(response);
    assert_eq!(club.join_style, Some(JoinStyle::Open));
    assert_eq!(club.tier(), Some("B+"));
    assert_eq!(club.members[2].daily_fans[0], -60_000_000);
}

#[tokio::test]
async fn fetches_circle_by_viewer_for_month() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("viewer_id", "100000001"))
        .and(query_param("year", "2026"))
        .and(query_param("month", "1"))
        .respond_with(json(CIRCLE))
        .expect(1)
        .mount(&server)
        .await;

    let request = CircleRequest::by_viewer_id(100000001).for_month(2026, 1);
    client(&server).circle(&request).await.unwrap();
}

#[tokio::test]
async fn rejects_invalid_month_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE))
        .expect(0)
        .mount(&server)
        .await;

    let request = CircleRequest::by_circle_id(1).for_month(2026, 13);
    let error = client(&server).circle(&request).await.unwrap_err();
    assert!(matches!(error, Error::InvalidRequest(_)));
}

#[tokio::test]
async fn lists_circles_with_filters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/circles/list"))
        .and(query_param("page", "1"))
        .and(query_param("limit", "2"))
        .and(query_param("min_members", "20"))
        .and(query_param("sort_by", "monthly_rank"))
        .and(query_param("sort_dir", "asc"))
        .and(query_param_is_missing("name"))
        .and(header("x-api-key", KEY))
        .respond_with(json(CIRCLE_LIST))
        .expect(1)
        .mount(&server)
        .await;

    let request = ListCirclesRequest {
        limit: Some(2),
        min_members: Some(20),
        sort_by: Some(CircleSort::MonthlyRank),
        sort_dir: Some(SortDir::Asc),
        ..Default::default()
    }
    .page(1);
    let page = client(&server).list_circles(&request).await.unwrap();
    assert_eq!(page.circles.len(), 2);
    assert_eq!(page.total, 5);
    assert!(page.has_next_page());
    assert_eq!(
        Club::from(page.circles[1].clone()).join_style,
        Some(JoinStyle::Closed)
    );
}

#[tokio::test]
async fn default_list_request_sends_no_filters() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .and(query_param_is_missing("page"))
        .and(query_param_is_missing("limit"))
        .and(query_param_is_missing("sort_by"))
        .respond_with(json(CIRCLE_LIST))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .list_circles(&ListCirclesRequest::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn returns_api_error_body() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(400).set_body_raw(
            r#"{"error":"Missing viewer_id or circle_id","status":400}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::BAD_REQUEST));
    match error {
        Error::Status {
            body: Some(body), ..
        } => {
            assert_eq!(body.error, "Missing viewer_id or circle_id");
            assert_eq!(body.details, None);
        }
        other => panic!("expected a status error with body, got {other:?}"),
    }
}

#[tokio::test]
async fn error_body_is_optional() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Status {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: None
        }
    ));
}

#[tokio::test]
async fn invalid_json_is_a_decode_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(r#"{"circle": "#))
        .mount(&server)
        .await;

    let error = client(&server)
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Decode(_)));
}

#[tokio::test]
async fn timeout_is_a_transport_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    let mut config = config(&server);
    config.timeout = Duration::from_millis(100);
    let error = Client::new(config)
        .unwrap()
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    match error {
        Error::Transport(e) => assert!(e.is_timeout()),
        other => panic!("expected a timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn debug_output_hides_the_key() {
    let server = MockServer::start().await;
    let client = client(&server);
    assert!(!format!("{client:?}").contains(KEY));
    assert!(!format!("{:?}", config(&server)).contains(KEY));
}

#[test]
fn rejects_invalid_config() {
    let bad_url = ClientConfig {
        base_url: "not a url".into(),
        ..ClientConfig::new(ApiKey::new(KEY))
    };
    assert!(matches!(Client::new(bad_url), Err(Error::Config(_))));

    let bad_key = ClientConfig::new(ApiKey::new("line\nbreak"));
    let error = Client::new(bad_key).unwrap_err();
    assert!(matches!(error, Error::Config(_)));
    assert!(!error.to_string().contains("line"));
}
