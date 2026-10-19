use super::*;
use reqwest::header::HeaderValue;

fn config() -> RetryConfig {
    RetryConfig {
        max_retries: 5,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
    }
}

#[test]
fn retryable_statuses() {
    for code in [429, 500, 502, 503, 504] {
        assert!(is_retryable_status(StatusCode::from_u16(code).unwrap()));
    }
    for code in [200, 400, 401, 403, 404, 501] {
        assert!(!is_retryable_status(StatusCode::from_u16(code).unwrap()));
    }
}

#[test]
fn delay_doubles_each_retry() {
    let c = config();
    assert_eq!(delay(&c, 0, None, 0.0), Duration::from_millis(50));
    assert_eq!(delay(&c, 1, None, 0.0), Duration::from_millis(100));
    assert_eq!(delay(&c, 2, None, 0.0), Duration::from_millis(200));
}

#[test]
fn jitter_adds_up_to_half() {
    let c = config();
    assert_eq!(delay(&c, 1, None, 0.5), Duration::from_millis(150));
    assert!(delay(&c, 1, None, 0.999) < Duration::from_millis(200));
}

#[test]
fn delay_is_capped() {
    let c = config();
    assert!(delay(&c, 30, None, 0.999) <= c.max_delay);
    assert_eq!(
        delay(&c, 0, Some(Duration::from_secs(60)), 0.0),
        c.max_delay
    );
}

#[test]
fn retry_after_overrides_backoff() {
    let c = config();
    assert_eq!(
        delay(&c, 3, Some(Duration::from_millis(10)), 0.9),
        Duration::from_millis(10)
    );
}

#[test]
fn parses_retry_after_seconds() {
    let mut headers = HeaderMap::new();
    assert_eq!(retry_after(&headers), None);
    headers.insert(RETRY_AFTER, HeaderValue::from_static("3"));
    assert_eq!(retry_after(&headers), Some(Duration::from_secs(3)));
    headers.insert(
        RETRY_AFTER,
        HeaderValue::from_static("Wed, 21 Oct 2026 07:28:00 GMT"),
    );
    assert_eq!(retry_after(&headers), None);
}
