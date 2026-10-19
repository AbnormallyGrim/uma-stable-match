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

#[test]
fn list_query_skips_unset_fields() {
    let value = serde_json::to_value(ListCirclesRequest::default()).unwrap();
    assert_eq!(value, serde_json::json!({}));
}

#[test]
fn list_query_uses_api_names() {
    let request = ListCirclesRequest {
        min_members: Some(20),
        sort_by: Some(CircleSort::MonthlyRank),
        sort_dir: Some(SortDir::Asc),
        ..Default::default()
    }
    .page(3);
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "page": 3,
            "min_members": 20,
            "sort_by": "monthly_rank",
            "sort_dir": "asc",
        })
    );
}
