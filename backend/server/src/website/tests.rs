use super::*;

use axum::body::Body;

fn sha256(content: &str) -> String {
    format!(
        "'sha256-{}'",
        STANDARD.encode(Sha256::digest(content.as_bytes()))
    )
}

#[test]
fn hashes_each_inline_element() {
    let html =
        "<head><SCRIPT>one()</SCRIPT><style>p{}</style><script type=module>\ntwo()\n</script>";
    assert_eq!(
        inline_hashes(html, "script"),
        [sha256("one()"), sha256("\ntwo()\n")]
    );
    assert_eq!(inline_hashes(html, "style"), [sha256("p{}")]);
}

#[test]
fn skips_external_and_empty_scripts_and_other_tags() {
    let html = "<script src=\"a.js\">x</script><script></script><scripts>y</scripts>";
    assert!(inline_hashes(html, "script").is_empty());
}

#[test]
fn stops_at_an_unclosed_element() {
    assert!(inline_hashes("<script>never closed", "script").is_empty());
    assert!(inline_hashes("<script", "script").is_empty());
}

#[test]
fn policy_adds_the_hashes_and_google_fonts() {
    let policy = content_security("<script>run()</script><style>p{}</style>");
    assert!(policy.contains(&format!(
        "script-src 'self' https://static.cloudflareinsights.com {};",
        sha256("run()")
    )));
    assert!(policy.contains(&format!(
        "style-src 'self' https://fonts.googleapis.com {};",
        sha256("p{}")
    )));
    assert!(policy.contains("font-src https://fonts.gstatic.com;"));
    assert!(!policy.contains("unsafe-inline"));
}

#[test]
fn policy_without_inline_elements_allows_the_site_and_the_beacon() {
    let policy = content_security("");
    assert!(policy.contains("script-src 'self' https://static.cloudflareinsights.com;"));
    assert!(policy.contains("connect-src 'self' https://cloudflareinsights.com;"));
    assert!(policy.contains("style-src 'self' https://fonts.googleapis.com;"));
}

#[test]
fn host_comes_from_the_header_or_the_authority() {
    let request = |builder: axum::http::request::Builder| builder.body(Body::empty()).unwrap();
    let with_header = request(Request::get("/").header(HOST, "example.test:8080"));
    assert_eq!(request_host(&with_header), Some("example.test"));
    let http2 = request(Request::get("https://example.test/page"));
    assert_eq!(request_host(&http2), Some("example.test"));
    assert_eq!(request_host(&request(Request::get("/"))), None);
}
