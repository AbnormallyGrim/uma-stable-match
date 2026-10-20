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

#[test]
fn page_keeps_other_fields() {
    let request = ListCirclesRequest {
        name: Some("Example".into()),
        page: Some(1),
        ..Default::default()
    };
    let next = request.page(2);
    assert_eq!(next.page, Some(2));
    assert_eq!(next.name.as_deref(), Some("Example"));
}

#[test]
fn gains_query_skips_unset_fields() {
    let value = serde_json::to_value(RankingsGainsRequest::default()).unwrap();
    assert_eq!(value, serde_json::json!({}));
}

#[test]
fn gains_query_uses_api_names() {
    for (sort, name) in [
        (GainsSort::Gain3d, "gain_3d"),
        (GainsSort::Gain7d, "gain_7d"),
        (GainsSort::Gain30d, "gain_30d"),
    ] {
        let request = RankingsGainsRequest {
            limit: Some(10),
            sort_by: Some(sort),
            ..RankingsGainsRequest::for_viewer(100000001)
        }
        .page(2);
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "page": 2,
                "limit": 10,
                "sort_by": name,
                "query": "100000001",
            })
        );
    }
}

#[test]
fn gains_page_keeps_other_fields() {
    let request = RankingsGainsRequest::for_viewer(100000001);
    let next = request.page(1);
    assert_eq!(next.page, Some(1));
    assert_eq!(next.query.as_deref(), Some("100000001"));
}
