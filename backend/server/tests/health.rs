//! Health route tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uma_stable_match_server::{router, AppState, Cache, PaceLookup, DEFAULT_PACE_TTL};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};

fn app(cache: Cache) -> AppState {
    let client = Client::new(ClientConfig {
        base_url: "http://127.0.0.1:9".into(),
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();
    AppState {
        cache,
        paces: PaceLookup::new(client, DEFAULT_PACE_TTL),
    }
}

#[tokio::test]
async fn health_returns_ok() {
    let response = router(app(Cache::new()))
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], br#"{"status":"ok"}"#);
}

#[tokio::test]
async fn unknown_route_is_not_found() {
    let response = router(app(Cache::new()))
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
