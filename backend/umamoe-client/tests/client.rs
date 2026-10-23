//! Public API tests against a mock server. All data is made up.

use std::time::{Duration, Instant};

use uma_stable_match_scoring::{Club, JoinStyle};
use uma_stable_match_umamoe_client::{
    ApiKey, CircleRequest, CircleSort, Client, ClientConfig, Error, GainsSort, ListCirclesRequest,
    Priority, RankingsGainsRequest, RankingsMonthlyRequest, RetryConfig, SortDir, StatusCode,
};
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "test-key";
const CIRCLE: &str = include_str!("fixtures/circle.json");
const CIRCLE_LIST: &str = include_str!("fixtures/circle_list.json");
const GAINS: &str = include_str!("fixtures/gains.json");

fn config(server: &MockServer) -> ClientConfig {
    ClientConfig {
        base_url: server.uri(),
        timeout: Duration::from_secs(5),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        },
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
async fn fetches_gains_for_a_viewer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/rankings/gains"))
        .and(query_param("query", "100000001"))
        .and(query_param("sort_by", "gain_7d"))
        .and(query_param_is_missing("page"))
        .and(query_param_is_missing("limit"))
        .and(header("x-api-key", KEY))
        .and(header("accept", "application/json"))
        .respond_with(json(GAINS))
        .expect(1)
        .mount(&server)
        .await;

    let request = RankingsGainsRequest {
        sort_by: Some(GainsSort::Gain7d),
        ..RankingsGainsRequest::for_viewer(100000001)
    };
    let response = client(&server).rankings_gains(&request).await.unwrap();
    assert_eq!(response.rankings.len(), 2);
    assert_eq!(response.sort_by.as_deref(), Some("gain_7d"));
    assert!(!response.has_next_page());

    let me = response.viewer(100000001).unwrap();
    assert_eq!(me.gain_7d, Some(14_000_000));
    assert_eq!(me.rank_30d, Some(115));
    assert_eq!(me.circle_id, Some(900001));
    assert_eq!(response.viewer(1000000012).unwrap().circle_id, None);
}

#[tokio::test]
async fn gains_errors_are_returned() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/rankings/gains"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .rankings_gains(&RankingsGainsRequest::for_viewer(100000001))
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::NOT_FOUND));
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
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
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
            status: StatusCode::NOT_FOUND,
            body: None
        }
    ));
}

#[tokio::test]
async fn retries_server_errors_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE))
        .expect(1)
        .mount(&server)
        .await;

    let response = client(&server)
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap();
    assert_eq!(response.circle.circle_id, 900001);
}

#[tokio::test]
async fn retries_rate_limited_requests() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
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
async fn gives_up_after_max_retries() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(500))
        .expect(3)
        .mount(&server)
        .await;

    let error = client(&server)
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::INTERNAL_SERVER_ERROR));
}

#[tokio::test]
async fn invalid_json_is_a_decode_error() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(r#"{"circle": "#))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Decode(_)));
}

#[tokio::test]
async fn retries_timeouts() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE).set_delay(Duration::from_secs(2)))
        .expect(2)
        .mount(&server)
        .await;

    let mut config = config(&server);
    config.timeout = Duration::from_millis(100);
    config.retry.max_retries = 1;
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

#[tokio::test]
async fn requests_are_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .expect(3)
        .mount(&server)
        .await;

    let mut config = config(&server);
    config.min_interval = Duration::from_millis(100);
    let client = Client::new(config).unwrap();
    let start = Instant::now();
    for _ in 0..3 {
        client
            .list_circles(&ListCirclesRequest::default())
            .await
            .unwrap();
    }
    assert!(start.elapsed() >= Duration::from_millis(200));
}

#[tokio::test]
async fn clones_share_the_rate_limit() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .expect(2)
        .mount(&server)
        .await;

    let mut config = config(&server);
    config.min_interval = Duration::from_millis(200);
    let client = Client::new(config).unwrap();
    let clone = client.clone();
    let request = ListCirclesRequest::default();
    let start = Instant::now();
    let (a, b) = tokio::join!(client.list_circles(&request), clone.list_circles(&request));
    a.unwrap();
    b.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(200));
}

#[tokio::test]
async fn interactive_requests_go_before_queued_background_ones() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE))
        .expect(1)
        .mount(&server)
        .await;

    let mut config = config(&server);
    config.min_interval = Duration::from_millis(200);
    let interactive = Client::new(config).unwrap();
    let background = interactive.with_priority(Priority::Background);

    let mut tasks = Vec::new();
    for page in 0..3 {
        let client = background.clone();
        tasks.push(tokio::spawn(async move {
            client
                .list_circles(&ListCirclesRequest::default().page(page))
                .await
                .unwrap();
        }));
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    interactive
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap();
    for task in tasks {
        task.await.unwrap();
    }

    let paths: Vec<_> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.url.path().to_owned())
        .collect();
    assert_eq!(
        paths,
        vec![
            "/api/v4/circles/list",
            "/api/v4/circles",
            "/api/v4/circles/list",
            "/api/v4/circles/list",
        ]
    );
}

#[test]
fn clients_default_to_interactive() {
    let client = Client::new(ClientConfig::new(ApiKey::new(KEY))).unwrap();
    assert!(format!("{client:?}").contains("Interactive"));
    let background = client.with_priority(Priority::Background);
    assert!(format!("{background:?}").contains("Background"));
}

#[tokio::test]
async fn retry_after_pauses_other_requests_too() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE))
        .mount(&server)
        .await;
    let interactive = Client::new(ClientConfig {
        retry: RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_secs(2),
        },
        ..config(&server)
    })
    .unwrap();
    let background = interactive.with_priority(Priority::Background);

    let started = Instant::now();
    let listing = tokio::spawn(async move {
        interactive
            .list_circles(&ListCirclesRequest::default())
            .await
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    background
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap();
    let waited = started.elapsed();
    listing.await.unwrap().unwrap();

    assert!(waited >= Duration::from_millis(900), "waited {waited:?}");
}

#[tokio::test]
async fn background_requests_wait_out_a_long_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(json(CIRCLE))
        .expect(1)
        .mount(&server)
        .await;

    let started = Instant::now();
    client(&server)
        .with_priority(Priority::Background)
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(900));
}

#[tokio::test]
async fn retry_after_is_waited_out_in_full_by_default() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .expect(1)
        .mount(&server)
        .await;

    let started = Instant::now();
    client(&server)
        .list_circles(&ListCirclesRequest::default())
        .await
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(900));
}

#[tokio::test]
async fn requests_with_a_max_pause_give_up_on_a_longer_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "60"))
        .expect(1)
        .mount(&server)
        .await;

    let started = Instant::now();
    let error = client(&server)
        .with_max_pause(Duration::from_secs(30))
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap_err();
    assert!(
        matches!(error, Error::RateLimited { retry_in } if retry_in == Duration::from_secs(60))
    );
    assert_eq!(error.status(), Some(StatusCode::TOO_MANY_REQUESTS));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn requests_with_a_max_pause_are_not_sent_during_a_longer_pause() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "60"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(json(CIRCLE_LIST))
        .expect(0)
        .mount(&server)
        .await;
    let visitor = client(&server).with_max_pause(Duration::from_secs(30));

    visitor
        .circle(&CircleRequest::by_circle_id(900001))
        .await
        .unwrap_err();
    let error = visitor
        .list_circles(&ListCirclesRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(error, Error::RateLimited { retry_in } if retry_in > Duration::from_secs(50)));
}

#[tokio::test]
async fn fetches_monthly_rankings_for_a_trainer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/rankings/monthly"))
        .and(query_param("query", "123456789"))
        .and(query_param("year", "2026"))
        .and(query_param("month", "2"))
        .and(header("x-api-key", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "rankings": [
                { "viewer_id": 1234567890, "circle_id": 5 },
                { "viewer_id": 123456789, "circle_id": 9, "circle_name": "Example Club" }
            ],
            "total": 2, "page": 0, "limit": 100, "total_pages": 1, "year": 2026, "month": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let response = client(&server)
        .rankings_monthly(&RankingsMonthlyRequest::for_viewer(123456789).for_month(2026, 2))
        .await
        .unwrap();
    assert_eq!(
        response.viewer(123456789).and_then(|r| r.circle_id),
        Some(9)
    );
    assert!(response.viewer(42).is_none());
}

#[tokio::test]
async fn monthly_rankings_reject_an_invalid_month() {
    let server = MockServer::start().await;
    let error = client(&server)
        .rankings_monthly(&RankingsMonthlyRequest::for_viewer(1).for_month(2026, 13))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRequest(_)));
}
