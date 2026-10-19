use super::*;
use axum::http::HeaderValue;

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn headers(values: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for v in values {
        headers.append("x-forwarded-for", HeaderValue::from_str(v).unwrap());
    }
    headers
}

#[test]
fn ipv4_is_its_own_group() {
    assert_eq!(group(ip("203.0.113.7")), ip("203.0.113.7"));
}

#[test]
fn ipv6_groups_by_64_bit_prefix() {
    assert_eq!(group(ip("2001:db8::1")), ip("2001:db8::"));
    assert_eq!(group(ip("2001:db8::ffff:1")), ip("2001:db8::"));
    assert_eq!(group(ip("2001:db8:0:1::1")), ip("2001:db8:0:1::"));
}

#[test]
fn mapped_ipv4_is_treated_as_ipv4() {
    assert_eq!(group(ip("::ffff:203.0.113.7")), ip("203.0.113.7"));
}

#[test]
fn forwarded_for_uses_the_rightmost_address() {
    assert_eq!(
        forwarded_for(&headers(&["203.0.113.7"])),
        Some(ip("203.0.113.7"))
    );
    assert_eq!(
        forwarded_for(&headers(&["198.51.100.1, 203.0.113.7"])),
        Some(ip("203.0.113.7"))
    );
    assert_eq!(
        forwarded_for(&headers(&["198.51.100.1", "203.0.113.9"])),
        Some(ip("203.0.113.9"))
    );
}

#[test]
fn bad_forwarded_for_is_ignored() {
    assert_eq!(forwarded_for(&HeaderMap::new()), None);
    assert_eq!(forwarded_for(&headers(&["unknown"])), None);
    assert_eq!(forwarded_for(&headers(&["203.0.113.7, "])), None);
}

#[test]
fn debug_hides_limiter_state() {
    let limits = RateLimits::new(30, 10, true);
    assert_eq!(
        format!("{limits:?}"),
        "RateLimits { trust_forwarded_for: true, .. }"
    );
}
