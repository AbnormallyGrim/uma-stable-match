//! Serving the project website on its own host name, with its own security policy.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, HOST, LOCATION};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::frontend;

const IMAGES: &str = "public, max-age=86400";
const REVALIDATE: &str = "no-cache";

/// A static website served for one host name, from `SITE_HOST` and `SITE_DIR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Website {
    /// Host name the website is served on, such as `umastable.com`.
    pub host: String,
    /// Directory with the website's files, including `index.html`.
    pub dir: PathBuf,
}

struct Routes {
    host: String,
    www: String,
    site: Router,
}

impl Website {
    /// Serves the website for requests to [`host`](Self::host), redirects `www.` plus the host to
    /// it, and passes every other request to `app`.
    pub fn wrap(&self, app: Router) -> Router {
        let html = fs::read_to_string(self.dir.join("index.html")).unwrap_or_else(|error| {
            tracing::warn!("could not read the website's index.html: {error}");
            String::new()
        });
        let policy =
            HeaderValue::from_str(&content_security(&html)).expect("the policy is printable ASCII");
        let routes = Arc::new(Routes {
            host: self.host.clone(),
            www: format!("www.{}", self.host),
            site: service(&self.dir, policy),
        });
        app.layer(from_fn_with_state(routes, dispatch))
    }
}

/// The `Content-Security-Policy` for the website: like the app's, plus Google Fonts and the inline
/// scripts and styles in `html`, allowed by their hashes.
pub fn content_security(html: &str) -> String {
    let with = |sources: &[&str], tag| {
        let mut sources: Vec<String> = sources.iter().map(|s| (*s).to_owned()).collect();
        sources.extend(inline_hashes(html, tag));
        sources.join(" ")
    };
    format!(
        "default-src 'self'; script-src {}; style-src {}; img-src 'self' data:; \
connect-src 'self'; font-src https://fonts.gstatic.com; object-src 'none'; base-uri 'self'; \
form-action 'self'; frame-ancestors 'none'",
        with(&["'self'"], "script"),
        with(&["'self'", "https://fonts.googleapis.com"], "style"),
    )
}

fn service(dir: &Path, policy: HeaderValue) -> Router {
    Router::new()
        .fallback_service(ServeDir::new(dir))
        .layer(from_fn(cache_headers))
        .layer(from_fn_with_state(policy, frontend::security_headers))
}

async fn dispatch(State(routes): State<Arc<Routes>>, request: Request, next: Next) -> Response {
    let Some(host) = request_host(&request) else {
        return next.run(request).await;
    };
    if host.eq_ignore_ascii_case(&routes.host) {
        match routes.site.clone().oneshot(request).await {
            Ok(response) => response,
            Err(never) => match never {},
        }
    } else if host.eq_ignore_ascii_case(&routes.www) {
        let path = request
            .uri()
            .path_and_query()
            .map_or("/", |path| path.as_str());
        let location = format!("https://{}{path}", routes.host);
        match HeaderValue::from_str(&location) {
            Ok(location) => {
                (StatusCode::PERMANENT_REDIRECT, [(LOCATION, location)]).into_response()
            }
            Err(_) => StatusCode::BAD_REQUEST.into_response(),
        }
    } else {
        next.run(request).await
    }
}

/// The host name a request is for, without the port: the `Host` header, or the URI's authority
/// for HTTP/2.
fn request_host(request: &Request) -> Option<&str> {
    let host = match request.headers().get(HOST) {
        Some(value) => value.to_str().ok()?,
        None => request.uri().authority()?.as_str(),
    };
    host.split(':').next()
}

async fn cache_headers(request: Request, next: Next) -> Response {
    let image = request.uri().path().starts_with("/images/");
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let value = if image { IMAGES } else { REVALIDATE };
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}

/// The `'sha256-…'` sources for each inline `tag` element in `html` with content, skipping
/// elements with a `src`.
fn inline_hashes(html: &str, tag: &str) -> Vec<String> {
    // Lowercasing ASCII keeps every byte offset, so offsets in `lower` work in `html`.
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut hashes = Vec::new();
    let mut from = 0;
    while let Some(start) = lower[from..].find(&open).map(|i| from + i) {
        let after = start + open.len();
        if !lower[after..].starts_with(|c: char| c == '>' || c.is_ascii_whitespace()) {
            from = after;
            continue;
        }
        let Some(body) = lower[after..].find('>').map(|i| after + i + 1) else {
            break;
        };
        let Some(end) = lower[body..].find(&close).map(|i| body + i) else {
            break;
        };
        let content = &html[body..end];
        if !lower[start..body].contains("src=") && !content.is_empty() {
            let digest = Sha256::digest(content.as_bytes());
            hashes.push(format!("'sha256-{}'", STANDARD.encode(digest)));
        }
        from = end + close.len();
    }
    hashes
}

#[cfg(test)]
mod tests;
