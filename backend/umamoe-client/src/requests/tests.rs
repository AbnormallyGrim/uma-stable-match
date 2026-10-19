use super::*;

#[test]
fn circle_query_by_id() {
    assert_eq!(
        CircleRequest::by_circle_id(42).query(),
        vec![("circle_id", "42".to_owned())]
    );
}

#[test]
fn circle_query_by_viewer_with_month() {
    assert_eq!(
        CircleRequest::by_viewer_id(1001).for_month(2026, 3).query(),
        vec![
            ("viewer_id", "1001".to_owned()),
            ("year", "2026".to_owned()),
            ("month", "3".to_owned()),
        ]
    );
}
