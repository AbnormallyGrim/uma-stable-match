//! Spans for a search. Kept in its own test binary, as tracing's scoped capture can miss spans when
//! other tests in the same binary trace at the same time. All data is made up.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use uma_stable_match_server::{
    app, run_scheduler, AppState, Cache, ClubDetails, DetailConfig, MatchSettings, PaceLookup,
    RateLimits, ScanConfig, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{
    ApiKey, Client, ClientConfig, ListCirclesRequest, RetryConfig,
};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TRAINER: u64 = 8100;

fn client(base_url: String) -> Client {
    Client::new(ClientConfig {
        base_url,
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 0,
            ..Default::default()
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap()
}

fn detail(id: u64, pace: i64) -> Value {
    let members: Vec<Value> = (0..10)
        .map(|i| {
            let mut fans = vec![0i64; 31];
            for (day, value) in fans.iter_mut().take(10).enumerate() {
                *value = 100_000_000 + (pace + 100_000 * i) * day as i64;
            }
            json!({ "viewer_id": 8000 + id * 100 + i as u64, "year": 2026, "month": 3, "daily_fans": fans })
        })
        .collect();
    json!({
        "circle": {
            "circle_id": id,
            "name": format!("Span Club {id}"),
            "member_count": 10,
            "join_style": 1,
            "monthly_rank": 100 + id,
            "monthly_point": 20_000_000,
            "last_updated": "2026-03-19T15:00:00Z",
        },
        "members": members,
    })
}

async fn scanned() -> (AppState, Client) {
    let server = MockServer::start().await;
    let clubs = [detail(1, 4_500_000), detail(2, 5_500_000)];
    let listed: Vec<Value> = clubs.iter().map(|d| d["circle"].clone()).collect();
    Mock::given(path("/api/v4/circles/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "circles": listed })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/rankings/monthly"))
        .and(query_param("query", TRAINER.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "rankings": [{ "viewer_id": TRAINER, "circle_id": 1 }] })),
        )
        .mount(&server)
        .await;
    for club in clubs {
        Mock::given(path("/api/v4/circles"))
            .and(query_param(
                "circle_id",
                club["circle"]["circle_id"].to_string(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(club))
            .mount(&server)
            .await;
    }

    let client = client(server.uri());
    let cache = Cache::new();
    let details = ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0;
    let (stop, stopped) = watch::channel(false);
    let handle = tokio::spawn(run_scheduler(
        client.clone(),
        details.clone(),
        ScanConfig::default(),
        Some(Duration::from_secs(3600)),
        cache.clone(),
        stopped,
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while cache.snapshot().is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("scan did not finish");
    stop.send(true).unwrap();
    handle.await.unwrap();
    let state = AppState {
        cache,
        details,
        matching: MatchSettings::default(),
        paces: PaceLookup::new(client.clone(), DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    };
    (state, client)
}

async fn get(state: &AppState, query: &str) -> StatusCode {
    app(state.clone(), None)
        .oneshot(
            Request::get(format!("/api/v1/recommendations{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[derive(Debug, Clone)]
struct Recorded {
    name: &'static str,
    parent: Option<&'static str>,
    fields: HashMap<&'static str, String>,
}

#[derive(Clone, Default)]
struct Spans(Arc<Mutex<HashMap<u64, Recorded>>>);

impl Spans {
    fn named(&self, name: &str) -> Vec<Recorded> {
        self.0
            .lock()
            .unwrap()
            .values()
            .filter(|span| span.name == name)
            .cloned()
            .collect()
    }

    fn all(&self) -> Vec<Recorded> {
        self.0.lock().unwrap().values().cloned().collect()
    }
}

struct Fields<'a>(&'a mut HashMap<&'static str, String>);

impl Visit for Fields<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name(), value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name(), format!("{value:?}"));
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Spans {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let parent = ctx
            .span(id)
            .and_then(|span| span.parent())
            .map(|parent| parent.name());
        let mut fields = HashMap::new();
        attrs.record(&mut Fields(&mut fields));
        let recorded = Recorded {
            name: attrs.metadata().name(),
            parent,
            fields,
        };
        self.0.lock().unwrap().insert(id.into_u64(), recorded);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, _: Context<'_, S>) {
        if let Some(span) = self.0.lock().unwrap().get_mut(&id.into_u64()) {
            values.record(&mut Fields(&mut span.fields));
        }
    }
}

fn capture() -> (Spans, tracing::subscriber::DefaultGuard) {
    let spans = Spans::default();
    let guard =
        tracing::subscriber::set_default(tracing_subscriber::registry().with(spans.clone()));
    (spans, guard)
}

#[tokio::test]
async fn a_search_has_a_span_per_step() {
    let (state, _) = scanned().await;
    let (spans, guard) = capture();
    assert_eq!(get(&state, "?pace=5000000").await, StatusCode::OK);
    drop(guard);

    let request = spans.named("request");
    assert_eq!(request.len(), 1);
    assert_eq!(request[0].fields["http.route"], "/api/v1/recommendations");
    assert_eq!(request[0].fields["http.response.status_code"], "200");

    let load = spans.named("load clubs");
    assert_eq!(load.len(), 1);
    assert_eq!(load[0].parent, Some("request"));
    assert_eq!(load[0].fields["shortlisted"], "2");
    assert_eq!(load[0].fields["pending"], "0");
    assert_eq!(spans.named("shortlist")[0].parent, Some("load clubs"));
    let score = spans.named("score");
    assert_eq!(score[0].parent, Some("request"));
    assert_eq!(score[0].fields["clubs"], "2");
}

#[tokio::test]
async fn a_trainer_search_shows_the_lookup() {
    let (state, _) = scanned().await;
    let (spans, guard) = capture();
    assert_eq!(
        get(&state, &format!("?trainer_id={TRAINER}")).await,
        StatusCode::OK
    );
    drop(guard);

    let lookup = spans.named("pace lookup");
    assert_eq!(lookup.len(), 1);
    assert_eq!(lookup[0].parent, Some("request"));
    assert_eq!(lookup[0].fields["source"], "umamoe");
    let calls = spans.named("umamoe");
    assert!(calls.len() >= 2);
    assert!(calls.iter().all(|call| call.parent == Some("pace lookup")));
    assert!(calls
        .iter()
        .any(|call| call.fields["endpoint"] == "/api/v4/rankings/monthly"));
    assert!(calls
        .iter()
        .all(|call| call.fields["http.response.status_code"] == "200"));
    assert_eq!(spans.named("your club")[0].parent, Some("request"));
    assert!(spans
        .all()
        .iter()
        .flat_map(|span| span.fields.values())
        .all(|value| !value.contains(&TRAINER.to_string())));
}

#[tokio::test]
async fn work_outside_a_request_starts_no_trace() {
    let (_, client) = scanned().await;
    let (spans, guard) = capture();
    client
        .list_circles(&ListCirclesRequest::default())
        .await
        .unwrap();
    drop(guard);
    assert!(spans.all().is_empty());
}
