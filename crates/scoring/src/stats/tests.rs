use super::*;

#[test]
fn median_odd_even_empty() {
    assert_eq!(median(&[]), None);
    assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
}

#[test]
fn gini_equal_and_unequal() {
    assert_eq!(gini(&[5.0, 5.0, 5.0, 5.0]), 0.0);
    assert_eq!(gini(&[1.0]), 0.0);
    assert_eq!(gini(&[0.0, 0.0]), 0.0);
    // One member does everything among four: (n - 1) / n
    assert!((gini(&[0.0, 0.0, 0.0, 10.0]) - 0.75).abs() < 1e-12);
    assert!((gini(&[1.0, 2.0, 3.0, 4.0]) - 0.25).abs() < 1e-12);
}

#[test]
fn rounding_sends_halves_to_even() {
    assert_eq!(round_half_even(0.5), 0.0);
    assert_eq!(round_half_even(1.5), 2.0);
    assert_eq!(round_half_even(2.5), 2.0);
    assert_eq!(round_half_even(72.5), 72.0);
    assert_eq!(round_half_even(73.5), 74.0);
    assert_eq!(round_half_even(72.4), 72.0);
    assert_eq!(round_half_even(72.6), 73.0);
}
