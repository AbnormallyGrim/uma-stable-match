use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uma_stable_match_server::router;

#[tokio::test]
async fn health_returns_ok() {
    let response = router()
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
    let response = router()
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
