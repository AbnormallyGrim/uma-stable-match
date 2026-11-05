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
