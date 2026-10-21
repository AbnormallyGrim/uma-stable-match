use super::*;
use axum::body::Body;

fn request(path: &str) -> Request {
    Request::get(path).body(Body::empty()).unwrap()
}

#[test]
fn unmatched_paths_get_a_fixed_label() {
    assert_eq!(route_label(&request("/api/v2/anything")), "/api (unknown)");
    assert_eq!(route_label(&request("/assets/app-made-up.js")), "frontend");
}

#[test]
fn timestamps_are_unix_seconds() {
    assert_eq!(
        unix_seconds(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
        1.7e9
    );
}
