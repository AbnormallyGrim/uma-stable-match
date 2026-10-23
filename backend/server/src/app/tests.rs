use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

use super::*;

fn with_time_limit(delay: Duration) -> Router {
    Router::new()
        .route(
            "/",
            get(move || async move {
                tokio::time::sleep(delay).await;
                "done"
            }),
        )
        .layer(from_fn(time_limit))
}

async fn call(router: Router) -> (StatusCode, String) {
    let response = router
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test(start_paused = true)]
async fn requests_within_the_limit_finish() {
    let (status, body) = call(with_time_limit(Duration::from_secs(29))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "done");
}

#[tokio::test(start_paused = true)]
async fn slow_requests_time_out_with_a_json_error() {
    let started = tokio::time::Instant::now();
    let (status, body) = call(with_time_limit(Duration::from_secs(60))).await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "error": "the request took too long; try again" })
    );
    assert_eq!(started.elapsed(), REQUEST_TIMEOUT);
}

#[tokio::test]
async fn panicking_requests_get_a_json_500() {
    let router = Router::new()
        .route(
            "/",
            get(|| async {
                if true {
                    panic!("made-up failure");
                }
                "unreachable"
            }),
        )
        .layer(CatchPanicLayer::custom(panic_response));
    let (status, body) = call(router).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "error": "internal error" })
    );
}
