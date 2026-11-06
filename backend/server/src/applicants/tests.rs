use super::*;

#[test]
fn gains_are_the_last_total_less_the_first() {
    assert_eq!(fans_this_month(&[100, 150, 400]), Some(300));
}

#[test]
fn days_without_data_and_previous_club_values_are_handled() {
    assert_eq!(fans_this_month(&[0, 100, 0, -250, 400]), Some(300));
}

#[test]
fn needs_two_days() {
    assert_eq!(fans_this_month(&[]), None);
    assert_eq!(fans_this_month(&[0, 0]), None);
    assert_eq!(fans_this_month(&[500]), None);
}

#[test]
fn alltime_is_copied_from_the_fan_history() {
    use uma_stable_match_umamoe_client::ProfileAlltime;
    let history = ProfileFanHistory {
        alltime: Some(ProfileAlltime {
            total_fans: 5_000,
            active_days: 10,
            avg_day: Some(500.0),
            rank_total_fans: Some(3),
            ..Default::default()
        }),
        ..Default::default()
    };
    let found = alltime(Some(history)).unwrap();
    assert_eq!(found.total_fans, 5_000);
    assert_eq!(found.avg_day, Some(500.0));
    assert_eq!(found.avg_week, None);
    assert_eq!(found.rank, Some(3));
    assert_eq!(alltime(None), None);
    assert_eq!(alltime(Some(ProfileFanHistory::default())), None);
}
