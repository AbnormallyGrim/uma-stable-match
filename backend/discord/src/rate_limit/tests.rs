use super::*;
use reqwest::header::HeaderValue;

fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        headers.insert(*name, HeaderValue::from_str(value).unwrap());
    }
    headers
}

#[test]
fn route_keys_keep_only_major_ids() {
    assert_eq!(
        route_key(&Method::GET, "/guilds/111/members/222"),
        "GET/guilds/111/members/:id"
    );
    assert_eq!(
        route_key(&Method::POST, "/channels/333/messages?wait=true"),
        "POST/channels/333/messages"
    );
    assert_eq!(route_key(&Method::GET, "/invites/abc"), "GET/invites/abc");
}

#[tokio::test(start_paused = true)]
async fn waits_for_an_empty_bucket() {
    let limits = RateLimits::default();
    limits.update(
        "GET/a",
        &headers(&[
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset-after", "2.5"),
        ]),
    );
    assert_eq!(limits.delay("GET/a"), Duration::from_millis(2500));
    assert_eq!(limits.delay("GET/b"), Duration::ZERO);

    let start = Instant::now();
    limits.wait("GET/a", Duration::from_secs(5)).await.unwrap();
    assert_eq!(start.elapsed(), Duration::from_millis(2500));
    assert_eq!(limits.delay("GET/a"), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn remaining_calls_go_straight_through() {
    let limits = RateLimits::default();
    limits.update(
        "GET/a",
        &headers(&[
            ("x-ratelimit-remaining", "3"),
            ("x-ratelimit-reset-after", "10"),
        ]),
    );
    assert_eq!(limits.delay("GET/a"), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn gives_up_on_long_waits() {
    let limits = RateLimits::default();
    limits.limited("GET/a", Duration::from_secs(60), false);
    let error = limits
        .wait("GET/a", Duration::from_secs(30))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::RateLimited(wait) if wait == Duration::from_secs(60)));
}

#[tokio::test(start_paused = true)]
async fn global_limits_pause_every_route() {
    let limits = RateLimits::default();
    limits.limited("GET/a", Duration::from_secs(1), true);
    assert_eq!(limits.delay("POST/b"), Duration::from_secs(1));
}

#[test]
fn reads_retry_after_from_the_body_or_header() {
    let (wait, global) = retry_after(
        &HeaderMap::new(),
        br#"{"message": "You are being rate limited.", "retry_after": 1.5, "global": true}"#,
    );
    assert_eq!(wait, Duration::from_millis(1500));
    assert!(global);

    let (wait, global) = retry_after(&headers(&[("retry-after", "3")]), b"");
    assert_eq!(wait, Duration::from_secs(3));
    assert!(!global);

    let (wait, _) = retry_after(&HeaderMap::new(), b"");
    assert_eq!(wait, Duration::from_secs(1));
}
