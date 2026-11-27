use super::*;

use axum::body::Body;
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;

fn header(html: &str) -> Option<String> {
    link_header(html).map(|value| value.to_str().unwrap().to_owned())
}

#[test]
fn preloads_a_vite_build() {
    let html = r#"<!doctype html><head>
<link rel="icon" type="image/png" href="/favicon-32.png" />
<script src="/theme-init.js"></script>
<script type="module" crossorigin src="/assets/index-abc.js"></script>
<link rel="stylesheet" crossorigin href="/assets/index-def.css">
</head>"#;
    assert_eq!(
        header(html).as_deref(),
        Some(
            "</theme-init.js>; rel=preload; as=script, \
</assets/index-abc.js>; rel=preload; as=script; crossorigin, \
</assets/index-def.css>; rel=preload; as=style; crossorigin"
        )
    );
}

#[test]
fn preconnects_but_skips_other_origins_and_inline_scripts() {
    let html = r#"<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter" rel="stylesheet">
<script src="//cdn.example.com/a.js"></script>
<script type="application/ld+json">{"a": "<b>"}</script>
<link rel=stylesheet href=/style.css>"#;
    assert_eq!(
        header(html).as_deref(),
        Some("<https://fonts.gstatic.com>; rel=preconnect; crossorigin, </style.css>; rel=preload; as=style")
    );
}

#[test]
fn no_header_without_assets() {
    assert_eq!(header("<p>hello</p>"), None);
    assert_eq!(header(""), None);
}

#[test]
fn site_pages_by_path() {
    let dir = std::env::temp_dir().join(format!("preload-site-{}", std::process::id()));
    fs::create_dir_all(dir.join("privacy")).unwrap();
    fs::write(dir.join("index.html"), r#"<script src="/a.js"></script>"#).unwrap();
    fs::write(
        dir.join("privacy/index.html"),
        r#"<link rel="stylesheet" href="/b.css">"#,
    )
    .unwrap();
    let preloads = Preloads::site(&dir);
    fs::remove_dir_all(&dir).unwrap();

    let at = |path| preloads.get(path).map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(at("/").as_deref(), Some("</a.js>; rel=preload; as=script"));
    assert_eq!(at("/index.html"), at("/"));
    assert_eq!(
        at("/privacy/").as_deref(),
        Some("</b.css>; rel=preload; as=style")
    );
    assert_eq!(at("/privacy/index.html"), at("/privacy/"));
    assert_eq!(at("/style.css"), None);
}

#[tokio::test]
async fn adds_the_header_to_html_only() {
    let preloads = Arc::new(Preloads {
        every_page: link_header(r#"<script src="/a.js"></script>"#),
        ..Preloads::default()
    });
    let app = Router::new()
        .route(
            "/",
            get(|| async { ([(CONTENT_TYPE, "text/html")], "page") }),
        )
        .route(
            "/a.js",
            get(|| async { ([(CONTENT_TYPE, "text/javascript")], "") }),
        )
        .route(
            "/missing",
            get(|| async { (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/html")], "") }),
        )
        .layer(from_fn_with_state(preloads, link_headers));
    let link = |path: &'static str| {
        let app = app.clone();
        async move {
            let request = Request::get(path).body(Body::empty()).unwrap();
            let response = app.oneshot(request).await.unwrap();
            response.headers().get(LINK).cloned()
        }
    };
    assert!(link("/").await.is_some());
    assert!(link("/a.js").await.is_none());
    assert!(link("/missing").await.is_none());
}
