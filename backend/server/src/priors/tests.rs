use super::*;
use uma_stable_match_scoring::MemberSnapshot;

const M: i64 = 1_000_000;

/// A member gaining `per_day` for `days` days in `month` of 2026.
fn member(viewer_id: u64, month: u32, per_day: i64, days: usize) -> MemberSnapshot {
    let fans = (0..days).map(|d| 100 * M + per_day * d as i64).collect();
    let mut member = MemberSnapshot::new(viewer_id, fans);
    member.year = Some(2026);
    member.month = Some(month);
    member
}

fn club(members: Vec<MemberSnapshot>) -> Club {
    let mut club = Club::new(1, "Prior Club");
    club.member_count = Some(members.len() as u32);
    club.members = members;
    club
}

#[test]
fn data_period_and_days_come_from_the_members() {
    let club = club(vec![member(1, 2, M, 28), member(2, 3, M, 2)]);
    assert_eq!(data_period(&club), Some((2026, 3)));
    assert_eq!(data_days(&club), 28);
    assert_eq!(data_period(&Club::new(1, "Empty")), None);
}

#[test]
fn month_before_wraps_the_year() {
    assert_eq!(month_before((2026, 3)), (2026, 2));
    assert_eq!(month_before((2026, 1)), (2025, 12));
}

#[tokio::test]
async fn members_get_their_own_pace_from_last_month() {
    let priors = Priors::default();
    let shared = SharedCache::none();
    let february = club(
        (1..=10)
            .map(|id| member(id, 2, id as i64 * M, 28))
            .collect(),
    );
    priors.capture(&shared, 1, &february).await;

    let mut march = club(vec![member(3, 3, 9 * M, 2)]);
    priors.attach(&shared, 1, &mut march, None).await;
    assert_eq!(march.members[0].prior, Some(3.0 * M as f64));
}

#[tokio::test]
async fn new_members_get_the_clubs_median_then_the_fallback() {
    let priors = Priors::default();
    let shared = SharedCache::none();
    let february = club((1..=10).map(|id| member(id, 2, 2 * M, 28)).collect());
    priors.capture(&shared, 1, &february).await;

    let mut march = club(vec![member(99, 3, 9 * M, 2)]);
    priors.attach(&shared, 1, &mut march, Some(5.0)).await;
    assert_eq!(march.members[0].prior, Some(2.0 * M as f64));

    let mut other = club(vec![member(99, 3, 9 * M, 2)]);
    priors.attach(&shared, 2, &mut other, Some(5.0)).await;
    assert_eq!(other.members[0].prior, Some(5.0));
}

#[tokio::test]
async fn kept_pool_medians_are_club_priors() {
    let priors = Priors::default();
    priors.set_club_medians((2026, 2), [(1, 4.0 * M as f64)]);

    let mut march = club(vec![member(99, 3, 9 * M, 2)]);
    priors
        .attach(&SharedCache::none(), 1, &mut march, None)
        .await;
    assert_eq!(march.members[0].prior, Some(4.0 * M as f64));
}

#[tokio::test]
async fn priors_from_another_month_are_not_used() {
    let priors = Priors::default();
    let shared = SharedCache::none();
    let january = club((1..=10).map(|id| member(id, 1, 2 * M, 28)).collect());
    priors.capture(&shared, 1, &january).await;

    let mut march = club(vec![member(1, 3, 9 * M, 2)]);
    priors.attach(&shared, 1, &mut march, None).await;
    assert_eq!(march.members[0].prior, None);
}
