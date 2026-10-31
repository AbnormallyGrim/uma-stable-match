//! Feature flag endpoint tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uma_stable_match_server::{
    app, app_with_features, AppState, Cache, ClubDetails, DetailConfig, Features, MatchSettings,
    PaceLookup, RateLimits, SharedPool, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};

fn state() -> AppState {
    let client = Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    AppState {
        cache: Cache::new(),
        details: ClubDetails::new(&client, SharedPool::default(), DetailConfig::default()).0,
        matching: MatchSettings::default(),
        human: Default::default(),
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
        limits: RateLimits::new(10_000, 10_000, false),
    }
}

async fn features(app: axum::Router) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::get("/api/v1/features")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn everything_is_off_by_default() {
    let (status, body) = features(app(state(), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "japanese": false, "web_analytics": null }));
}

#[tokio::test]
async fn reports_the_japanese_flag_when_on() {
    let features_on = Features {
        japanese: true,
        ..Features::default()
    };
    let (status, body) = features(app_with_features(state(), None, features_on)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "japanese": true, "web_analytics": null }));
}

#[tokio::test]
async fn reports_the_web_analytics_token_when_set() {
    let features_on = Features {
        web_analytics: Some("madeuptoken123".into()),
        ..Features::default()
    };
    let (_, body) = features(app_with_features(state(), None, features_on)).await;
    assert_eq!(body["web_analytics"], "madeuptoken123");
}
