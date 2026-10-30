use super::*;

fn member(id: u64, daily_fans: &[i64]) -> MemberSnapshot {
    MemberSnapshot {
        viewer_id: id,
        year: Some(2026),
        month: Some(10),
        daily_fans: daily_fans.to_vec(),
        prior: None,
    }
}

#[test]
fn pace_skips_empty_days_and_counts_previous_club_fans() {
    assert_eq!(daily_pace(&[-100, -110, 120, 130]), Some(10.0));
    assert_eq!(daily_pace(&[0, 100, 0, 130, 160, 0]), Some(30.0));
    assert_eq!(daily_pace(&[]), None);
    assert_eq!(daily_pace(&[100, 0, 0, 200]), None);
}

#[test]
fn pace_can_be_zero() {
    assert_eq!(daily_pace(&[500, 500, 500]), Some(0.0));
}

#[test]
fn current_members_drop_people_who_left() {
    let members = vec![
        member(1, &[10, 20, 30, 40]),
        member(2, &[10, 20, 0, 0]), // left after day 2
        member(3, &[0, 0, 5, 6]),   // joined on day 3
    ];
    let ids: Vec<u64> = current_members(&members)
        .iter()
        .map(|m| m.viewer_id)
        .collect();
    assert_eq!(ids, vec![1, 3]);
}

#[test]
fn previous_club_value_on_latest_day_is_not_current() {
    let members = vec![member(1, &[10, 20, 30]), member(2, &[-5, -6, -7])];
    let ids: Vec<u64> = current_members(&members)
        .iter()
        .map(|m| m.viewer_id)
        .collect();
    assert_eq!(ids, vec![1]);
}

#[test]
fn only_latest_month_counts() {
    let mut old = member(9, &[10, 20, 30, 40]);
    old.month = Some(9);
    let members = vec![old, member(1, &[10, 20])];
    let ids: Vec<u64> = current_members(&members)
        .iter()
        .map(|m| m.viewer_id)
        .collect();
    assert_eq!(ids, vec![1]);
}

#[test]
fn members_without_period_are_all_considered() {
    let mut a = member(1, &[1, 2, 3]);
    let mut b = member(2, &[1, 2, 0]);
    a.year = None;
    a.month = None;
    b.year = None;
    b.month = None;
    let ids: Vec<u64> = current_members(&[a, b])
        .iter()
        .map(|m| m.viewer_id)
        .collect();
    assert_eq!(ids, vec![1]);
}

#[test]
fn empty_club_has_no_current_members() {
    assert!(current_members(&[]).is_empty());
    assert!(current_members(&[member(1, &[0, 0])]).is_empty());
}

#[test]
fn blend_shifts_weight_to_this_month_with_each_gain() {
    // Gains of 10 a day this month against a prior of 40.
    assert_eq!(blended_pace(&[100], Some(40.0), 1.0), Some(40.0));
    assert_eq!(blended_pace(&[100, 110], Some(40.0), 1.0), Some(25.0));
    assert_eq!(
        blended_pace(&[100, 110, 120, 130], Some(40.0), 1.0),
        Some(17.5)
    );
    assert_eq!(
        blended_pace(&[100, 110, 120, 130], Some(40.0), 3.0),
        Some(25.0)
    );
}

#[test]
fn blend_skips_empty_days_and_counts_previous_club_fans() {
    assert_eq!(blended_pace(&[-100, 0, 120], Some(0.0), 0.0), Some(20.0));
}

#[test]
fn blend_without_a_prior_is_the_daily_pace() {
    assert_eq!(blended_pace(&[100, 110, 120], None, 4.0), Some(10.0));
    assert_eq!(blended_pace(&[100, 110], None, 4.0), None);
    assert_eq!(
        blended_pace(&[100, 110, 120], Some(f64::NAN), 4.0),
        Some(10.0)
    );
}

#[test]
fn blend_needs_a_valid_k() {
    assert_eq!(blended_pace(&[100, 110], Some(40.0), -1.0), None);
    assert_eq!(blended_pace(&[100, 110], Some(40.0), f64::INFINITY), None);
    assert_eq!(blended_pace(&[100, 110], Some(40.0), 0.0), Some(10.0));
}

#[test]
fn blend_of_no_data_is_the_prior() {
    assert_eq!(blended_pace(&[], Some(40.0), 1.0), Some(40.0));
    assert_eq!(blended_pace(&[], None, 1.0), None);
}
