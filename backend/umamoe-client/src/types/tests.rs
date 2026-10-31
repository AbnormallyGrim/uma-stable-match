use super::*;

fn member(viewer_id: u64, daily_fans: Vec<i64>) -> CircleMember {
    CircleMember {
        viewer_id,
        year: Some(2026),
        month: Some(1),
        daily_fans,
        ..Default::default()
    }
}

#[test]
fn missing_fields_take_defaults() {
    let r: CircleResponse = serde_json::from_str(r#"{"circle":{"circle_id":7}}"#).unwrap();
    assert_eq!(r.circle.circle_id, 7);
    assert_eq!(r.circle.name, "");
    assert!(r.members.is_empty());
    assert_eq!(r.club_rank, None);
}

#[test]
fn unknown_fields_are_ignored() {
    let c: Circle = serde_json::from_str(r#"{"circle_id":1,"new_field":true}"#).unwrap();
    assert_eq!(c.circle_id, 1);
}

#[test]
fn nulls_become_none() {
    let c: Circle = serde_json::from_str(r#"{"circle_id":1,"join_style":null}"#).unwrap();
    assert_eq!(c.join_style, None);
}

#[test]
fn circle_converts_to_club() {
    let club = Club::from(Circle {
        circle_id: 42,
        name: "Example Club".into(),
        member_count: Some(25),
        join_style: Some(2),
        monthly_rank: Some(120),
        last_month_rank: Some(150),
        policy: Some(3),
        comment: Some("Hello".into()),
        ..Default::default()
    });
    assert_eq!(club.circle_id, 42);
    assert_eq!(club.name, "Example Club");
    assert_eq!(club.member_count, Some(25));
    assert_eq!(club.join_style, Some(JoinStyle::Approval));
    assert_eq!(club.monthly_rank, Some(120));
    assert_eq!(club.last_month_rank, Some(150));
    assert_eq!(club.policy, Some(3));
    assert_eq!(club.comment.as_deref(), Some("Hello"));
    assert_eq!(club.club_rank, None);
    assert!(club.members.is_empty());
}

#[test]
fn unknown_or_negative_codes_become_none() {
    let club = Club::from(Circle {
        join_style: Some(9),
        member_count: Some(-1),
        ..Default::default()
    });
    assert_eq!(club.join_style, None);
    assert_eq!(club.member_count, None);

    let club = Club::from(CircleResponse {
        club_rank: Some(300),
        ..Default::default()
    });
    assert_eq!(club.club_rank, None);
}

#[test]
fn response_converts_with_members_and_tier() {
    let club = Club::from(CircleResponse {
        circle: Circle {
            circle_id: 42,
            ..Default::default()
        },
        members: vec![
            CircleMember {
                trainer_name: Some("Made-up Trainer".into()),
                ..member(1001, vec![10, 20, 30])
            },
            member(1002, vec![-5, 0, 8]),
        ],
        club_rank: Some(8),
        ..Default::default()
    });
    assert_eq!(club.club_rank, Some(8));
    assert_eq!(club.tier(), Some("A+"));
    assert_eq!(club.members.len(), 2);
    assert_eq!(club.members[1].viewer_id, 1002);
    assert_eq!(club.members[1].daily_fans, vec![-5, 0, 8]);
    assert_eq!(club.members[0].year, Some(2026));
    assert_eq!(club.members[0].month, Some(1));
    assert_eq!(club.members[0].name.as_deref(), Some("Made-up Trainer"));
    assert_eq!(club.members[1].name, None);
}

#[test]
fn has_next_page() {
    let page = |page, total_pages| CircleListResponse {
        page,
        total_pages,
        ..Default::default()
    };
    assert!(page(0, 2).has_next_page());
    assert!(!page(1, 2).has_next_page());
    assert!(!page(0, 0).has_next_page());
}

#[test]
fn gains_missing_and_null_fields_become_none() {
    let r: GainsRanking =
        serde_json::from_str(r#"{"viewer_id":100000001,"gain_7d":null,"new_field":1}"#).unwrap();
    assert_eq!(r.viewer_id, 100000001);
    assert_eq!(r.gain_7d, None);
    assert_eq!(r.gain_30d, None);
    assert_eq!(r.circle_id, None);
}

#[test]
fn gains_has_next_page() {
    let page = |page, total_pages| GainsRankingsResponse {
        page,
        total_pages,
        ..Default::default()
    };
    assert!(page(0, 2).has_next_page());
    assert!(!page(1, 2).has_next_page());
    assert!(!page(0, 0).has_next_page());
}

#[test]
fn gains_finds_an_exact_viewer() {
    let ranking = |viewer_id| GainsRanking {
        viewer_id,
        ..Default::default()
    };
    let response = GainsRankingsResponse {
        rankings: vec![ranking(1000000012), ranking(100000001)],
        ..Default::default()
    };
    assert_eq!(
        response.viewer(100000001).map(|r| r.viewer_id),
        Some(100000001)
    );
    assert!(response.viewer(100000002).is_none());
}
