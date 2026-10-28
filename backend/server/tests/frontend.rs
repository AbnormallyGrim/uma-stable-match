//! Frontend serving and security header tests against a temporary directory.

use std::fs;
use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uma_stable_match_server::{
    app, AppState, Cache, ClubDetails, DetailConfig, MatchSettings, PaceLookup, RateLimits,
    SharedPool, CONTENT_SECURITY, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};

const INDEX: &str = "<!doctype html><title>Uma Stable Match</title><div id=app></div>";
const SCRIPT: &str = "console.log('made-up bundle');";

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

fn frontend() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.html"), INDEX).unwrap();
    fs::write(dir.path().join("theme-init.js"), "void 0;").unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/index-abc123.js"), SCRIPT).unwrap();
    dir
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

async fn get(app: &Router, path: &str) -> Reply {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}

fn with_frontend(dir: &Path) -> Router {
    app(state(), Some(dir))
}

#[tokio::test]
async fn serves_index_without_long_caching() {
    let dir = frontend();
    let reply = get(&with_frontend(dir.path()), "/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, INDEX);
    assert_eq!(reply.headers["content-type"], "text/html");
    assert_eq!(reply.headers["cache-control"], "no-cache");
}

#[tokio::test]
async fn caches_hashed_assets_for_a_year() {
    let dir = frontend();
    let reply = get(&with_frontend(dir.path()), "/assets/index-abc123.js").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, SCRIPT);
    assert_eq!(reply.headers["content-type"], "text/javascript");
    assert_eq!(
        reply.headers["cache-control"],
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn revalidates_unhashed_files() {
    let dir = frontend();
    let reply = get(&with_frontend(dir.path()), "/theme-init.js").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["cache-control"], "no-cache");
}

#[tokio::test]
async fn falls_back_to_index_for_page_routes() {
    let dir = frontend();
    let app = with_frontend(dir.path());
    for path in ["/results", "/some/page", "/?pace=7600000"] {
        let reply = get(&app, path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert_eq!(reply.body, INDEX, "{path}");
        assert_eq!(reply.headers["cache-control"], "no-cache", "{path}");
    }
}

#[tokio::test]
async fn missing_files_are_not_found() {
    let dir = frontend();
    let app = with_frontend(dir.path());
    for path in ["/assets/missing.js", "/favicon.ico"] {
        let reply = get(&app, path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(!reply.body.contains("<title>"), "{path}");
        assert!(reply.headers.get("cache-control").is_none(), "{path}");
    }
}

#[tokio::test]
async fn api_routes_are_never_shadowed() {
    let dir = frontend();
    let app = with_frontend(dir.path());

    let health = get(&app, "/api/v1/health").await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.body, r#"{"status":"ok"}"#);

    for path in ["/api", "/api/v1/unknown", "/api/v2/recommendations"] {
        let reply = get(&app, path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(reply.body, r#"{"error":"not found"}"#, "{path}");
    }
}

#[tokio::test]
async fn adds_security_headers_everywhere() {
    let dir = frontend();
    let app = with_frontend(dir.path());
    for path in [
        "/",
        "/assets/index-abc123.js",
        "/api/v1/health",
        "/api/nope",
        "/missing.txt",
    ] {
        let reply = get(&app, path).await;
        assert_eq!(
            reply.headers["content-security-policy"], CONTENT_SECURITY,
            "{path}"
        );
        assert_eq!(reply.headers["x-content-type-options"], "nosniff", "{path}");
        assert_eq!(reply.headers["referrer-policy"], "no-referrer", "{path}");
        assert_eq!(reply.headers["x-frame-options"], "DENY", "{path}");
        assert_eq!(
            reply.headers["strict-transport-security"], "max-age=31536000",
            "{path}"
        );
    }
}

#[tokio::test]
async fn csp_allows_only_same_origin_and_inline_data_images() {
    assert!(CONTENT_SECURITY.contains("script-src 'self'"));
    assert!(!CONTENT_SECURITY.contains("unsafe-inline"));
    assert!(CONTENT_SECURITY.contains("img-src 'self' data:"));
    assert!(CONTENT_SECURITY.contains("frame-ancestors 'none'"));
}

#[tokio::test]
async fn runs_api_only_without_a_frontend() {
    let app = app(state(), None);
    let root = get(&app, "/").await;
    assert_eq!(root.status, StatusCode::NOT_FOUND);
    assert_eq!(root.headers["x-content-type-options"], "nosniff");

    let health = get(&app, "/api/v1/health").await;
    assert_eq!(health.status, StatusCode::OK);
}
