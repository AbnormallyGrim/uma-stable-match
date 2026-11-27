//! `Link` headers that list a page's stylesheets, scripts and preconnects, for Cloudflare's Early
//! Hints.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{CONTENT_TYPE, LINK};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

/// The `Link` header for each HTML page, by request path.
#[derive(Debug, Default)]
pub(crate) struct Preloads {
    pages: HashMap<String, HeaderValue>,
    every_page: Option<HeaderValue>,
}

impl Preloads {
    /// One header, from `index`, for every HTML response, as the app serves `index.html` for its
    /// routes.
    pub(crate) fn single(index: &Path) -> Self {
        Self {
            every_page: fs::read_to_string(index)
                .ok()
                .and_then(|html| link_header(&html)),
            ..Self::default()
        }
    }

    /// A header for each `.html` file under `dir`, by its path, and each `index.html` also by its
    /// directory's path with a trailing `/`.
    pub(crate) fn site(dir: &Path) -> Self {
        let mut pages = HashMap::new();
        collect(dir, "/", &mut pages);
        Self {
            pages,
            every_page: None,
        }
    }

    fn get(&self, path: &str) -> Option<&HeaderValue> {
        self.pages.get(path).or(self.every_page.as_ref())
    }
}

fn collect(dir: &Path, prefix: &str, pages: &mut HashMap<String, HeaderValue>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if path.is_dir() {
            collect(&path, &format!("{prefix}{name}/"), pages);
        } else if name.ends_with(".html") {
            let Some(link) = fs::read_to_string(&path)
                .ok()
                .and_then(|html| link_header(&html))
            else {
                continue;
            };
            if name == "index.html" {
                pages.insert(prefix.to_owned(), link.clone());
            }
            pages.insert(format!("{prefix}{name}"), link);
        }
    }
}

/// Adds the page's `Link` header to successful HTML responses.
pub(crate) async fn link_headers(
    State(preloads): State<Arc<Preloads>>,
    request: Request,
    next: Next,
) -> Response {
    let link = preloads.get(request.uri().path()).cloned();
    let mut response = next.run(request).await;
    let html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if let Some(link) = link.filter(|_| html && response.status().is_success()) {
        response.headers_mut().insert(LINK, link);
    }
    response
}

/// The `Link` header for `html`: a preload for each same-origin stylesheet and script, and each
/// preconnect. `None` if there are none.
pub(crate) fn link_header(html: &str) -> Option<HeaderValue> {
    let mut links = Vec::new();
    for (tag, attributes) in start_tags(html) {
        let get = |name: &str| {
            attributes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        let (url, kind) = match tag.as_str() {
            "script" => (get("src"), "rel=preload; as=script"),
            "link" => match get("rel").map(str::to_ascii_lowercase).as_deref() {
                Some("stylesheet") => (get("href"), "rel=preload; as=style"),
                Some("preconnect") => (get("href"), "rel=preconnect"),
                _ => continue,
            },
            _ => continue,
        };
        let Some(url) = url else { continue };
        let preconnect = kind == "rel=preconnect";
        let same_origin = url.starts_with('/') && !url.starts_with("//");
        if (!preconnect && !same_origin) || url.contains(['<', '>', ',', ';']) {
            continue;
        }
        let crossorigin = if get("crossorigin").is_some() {
            "; crossorigin"
        } else {
            ""
        };
        links.push(format!("<{url}>; {kind}{crossorigin}"));
    }
    if links.is_empty() {
        return None;
    }
    HeaderValue::from_str(&links.join(", ")).ok()
}

/// Each `<link>` and `<script>` start tag in `html`, with its attributes, names lowercased.
fn start_tags(html: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut tags = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let name_end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        if name != "link" && name != "script" {
            continue;
        }
        let (attributes, after) = attributes(&rest[name_end..]);
        tags.push((name, attributes));
        rest = after;
    }
    tags
}

/// The attributes at the start of `input`, up to the tag's `>`, and what follows it.
fn attributes(mut input: &str) -> (Vec<(String, String)>, &str) {
    let mut attributes = Vec::new();
    loop {
        input = input.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if input.is_empty() {
            return (attributes, input);
        }
        if let Some(after) = input.strip_prefix('>') {
            return (attributes, after);
        }
        let name_end = input
            .find(|c: char| c.is_ascii_whitespace() || c == '=' || c == '>' || c == '/')
            .unwrap_or(input.len())
            .max(1);
        let name = input[..name_end].to_ascii_lowercase();
        input = input[name_end..].trim_start();
        let mut value = String::new();
        if let Some(after) = input.strip_prefix('=') {
            let after = after.trim_start();
            let (found, remaining) = match after.chars().next() {
                Some(quote @ ('"' | '\'')) => {
                    let body = &after[1..];
                    let end = body.find(quote).unwrap_or(body.len());
                    (&body[..end], body.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = after
                        .find(|c: char| c.is_ascii_whitespace() || c == '>')
                        .unwrap_or(after.len());
                    after.split_at(end)
                }
            };
            value = found.to_owned();
            input = remaining;
        }
        attributes.push((name, value));
    }
}

#[cfg(test)]
mod tests;
