//! Serving the built frontend, and the security headers on every response.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

/// Builds a `Content-Security-Policy`, adding extra script and connect sources.
macro_rules! content_security {
    ($script:literal, $connect:literal) => {
        concat!(
            "default-src 'self'; \
script-src 'self' https://challenges.cloudflare.com",
            $script,
            "; style-src 'self'; img-src 'self' data:; connect-src 'self'",
            $connect,
            "; font-src 'self'; frame-src https://challenges.cloudflare.com https://strawpoll.com; \
object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'"
        )
    };
}

/// The `Content-Security-Policy` header sent with every response.
pub const CONTENT_SECURITY: &str = content_security!("", "");

/// [`CONTENT_SECURITY`] plus Cloudflare Web Analytics, sent when its token is set.
pub const CONTENT_SECURITY_WITH_ANALYTICS: &str = content_security!(
    " https://static.cloudflareinsights.com",
    " https://cloudflareinsights.com"
);

const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const REVALIDATE: &str = "no-cache";

pub(crate) fn service(dir: &Path) -> Router {
    let index = Arc::new(dir.join("index.html"));
    let fallback = axum::routing::any(spa_fallback).with_state(index);
    Router::new()
        .fallback_service(ServeDir::new(dir).fallback(fallback))
        .layer(from_fn(cache_headers))
}

async fn spa_fallback(State(index): State<Arc<PathBuf>>, request: Request) -> Response {
    if has_extension(request.uri().path()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match ServeFile::new(index.as_path()).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(never) => match never {},
    }
}

fn has_extension(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'))
}

async fn cache_headers(request: Request, next: Next) -> Response {
    let hashed = request.uri().path().starts_with("/assets/");
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let value = if hashed { IMMUTABLE } else { REVALIDATE };
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}

/// Adds the security headers, with `content_security` as the `Content-Security-Policy`.
pub(crate) async fn security_headers(
    State(content_security): State<HeaderValue>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers
        .entry(CONTENT_SECURITY_POLICY)
        .or_insert(content_security);
    for (name, value) in [
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (REFERRER_POLICY, "no-referrer"),
        (X_FRAME_OPTIONS, "DENY"),
        (STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
    ] {
        headers
            .entry(name)
            .or_insert(HeaderValue::from_static(value));
    }
    response
}

#[cfg(test)]
mod tests;
