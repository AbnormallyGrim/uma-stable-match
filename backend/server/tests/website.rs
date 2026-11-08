//! Website serving tests: host routing, the `www.` redirect and the website's own headers.

use std::fs;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uma_stable_match_server::{
    app, website_content_security, AppState, Cache, ClubDetails, DetailConfig, MatchSettings,
    PaceLookup, RateLimits, SharedPool, Website, CONTENT_SECURITY, DEFAULT_PACE_TTL,
};
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig};

const HOST: &str = "example.test";
const SITE: &str = "<!doctype html><title>Made-up site</title><script>void 0;</script>";
const APP: &str = "<!doctype html><title>Made-up app</title><div id=app></div>";

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

struct Dirs {
    site: tempfile::TempDir,
    app: tempfile::TempDir,
}

fn dirs() -> Dirs {
    let site = tempfile::tempdir().unwrap();
    fs::write(site.path().join("index.html"), SITE).unwrap();
    fs::write(site.path().join("index.md"), "# Made-up site").unwrap();
    fs::create_dir(site.path().join("images")).unwrap();
    fs::write(site.path().join("images/logo.webp"), "made-up image").unwrap();
    let app = tempfile::tempdir().unwrap();
    fs::write(app.path().join("index.html"), APP).unwrap();
    Dirs { site, app }
}

fn server(dirs: &Dirs) -> Router {
    let website = Website {
        host: HOST.into(),
        dir: dirs.site.path().to_owned(),
    };
    website.wrap(app(state(), Some(dirs.app.path())))
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

async fn get(app: &Router, host: Option<&str>, path: &str) -> Reply {
    let mut request = Request::get(path);
    if let Some(host) = host {
        request = request.header("host", host);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
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

#[tokio::test]
async fn serves_the_website_with_its_own_policy() {
    let dirs = dirs();
    let reply = get(&server(&dirs), Some(HOST), "/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, SITE);
    assert_eq!(reply.headers["cache-control"], "no-cache");
    assert_eq!(
        reply.headers["content-security-policy"],
        website_content_security(SITE).as_str()
    );
    assert_ne!(reply.headers["content-security-policy"], CONTENT_SECURITY);
    assert_eq!(reply.headers["x-content-type-options"], "nosniff");
    assert_eq!(reply.headers["x-frame-options"], "DENY");
    assert_eq!(
        reply.headers["strict-transport-security"],
        "max-age=31536000"
    );
}

#[tokio::test]
async fn matches_the_host_whatever_its_case_or_port() {
    let dirs = dirs();
    let reply = get(&server(&dirs), Some("Example.Test:8080"), "/").await;
    assert_eq!(reply.body, SITE);
}

#[tokio::test]
async fn caches_images_for_a_day_and_serves_markdown() {
    let dirs = dirs();
    let app = server(&dirs);
    let image = get(&app, Some(HOST), "/images/logo.webp").await;
    assert_eq!(image.status, StatusCode::OK);
    assert_eq!(image.headers["cache-control"], "public, max-age=86400");

    let markdown = get(&app, Some(HOST), "/index.md").await;
    assert_eq!(markdown.status, StatusCode::OK);
    assert_eq!(markdown.headers["content-type"], "text/markdown");
    assert_eq!(markdown.headers["cache-control"], "no-cache");
}

#[tokio::test]
async fn website_has_no_app_routes_or_page_fallback() {
    let dirs = dirs();
    let app = server(&dirs);
    for path in ["/missing", "/missing.png", "/api/v1/health", "/clubs/123"] {
        let reply = get(&app, Some(HOST), path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(reply.headers.get("cache-control").is_none(), "{path}");
    }
}

#[tokio::test]
async fn redirects_www_to_the_host() {
    let dirs = dirs();
    let reply = get(
        &server(&dirs),
        Some("www.example.test"),
        "/index.md?lang=ja",
    )
    .await;
    assert_eq!(reply.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        reply.headers["location"],
        "https://example.test/index.md?lang=ja"
    );
}

#[tokio::test]
async fn other_hosts_get_the_app() {
    let dirs = dirs();
    let app = server(&dirs);
    for host in [Some("app.example.test"), Some("127.0.0.1:8080"), None] {
        let page = get(&app, host, "/").await;
        assert_eq!(page.body, APP, "{host:?}");
        assert_eq!(
            page.headers["content-security-policy"], CONTENT_SECURITY,
            "{host:?}"
        );
        let health = get(&app, host, "/api/v1/health").await;
        assert_eq!(health.status, StatusCode::OK, "{host:?}");
    }
}
