use uma_stable_match_scoring::{JoinStyle, MemberSnapshot};

use super::*;

fn club(daily_gains: &[i64]) -> Club {
    let mut club = Club::new(1, "Example Club");
    club.member_count = Some(daily_gains.len() as u32);
    club.join_style = Some(JoinStyle::Closed);
    club.members = daily_gains
        .iter()
        .enumerate()
        .map(|(i, gain)| {
            let start = 100_000_000;
            let fans = (0..4).map(|day| start + gain * day).collect();
            let mut member = MemberSnapshot::new(7001 + i as u64, fans);
            member.year = Some(2026);
            member.month = Some(3);
            member
        })
        .collect();
    club
}

#[test]
fn defaults_to_no_correction() {
    let c = Calibration::default();
    assert_eq!(c.factor(), 1.0);
    assert_eq!(c.samples(), 0);
    assert!(!c.is_calibrated());
}

#[test]
fn too_few_samples_keep_the_default() {
    let c = Calibration::from_ratios(std::iter::repeat_n(0.8, MIN_CALIBRATION_SAMPLES - 1));
    assert_eq!(c.factor(), 1.0);
    assert_eq!(c.samples(), MIN_CALIBRATION_SAMPLES - 1);
    assert!(!c.is_calibrated());
}

#[test]
fn median_of_ratios() {
    let c = Calibration::from_ratios((1..=21).map(|i| f64::from(i) / 10.0));
    assert_eq!(c.factor(), 1.1);
    assert_eq!(c.samples(), 21);
    assert!(c.is_calibrated());

    let even = Calibration::from_ratios((1..=20).map(|i| f64::from(i) / 10.0));
    assert!((even.factor() - 1.05).abs() < 1e-12);
}

#[test]
fn outliers_barely_move_the_factor() {
    let mut ratios = vec![0.8; 19];
    ratios.extend([0.01, 50.0, 1e9]);
    assert_eq!(Calibration::from_ratios(ratios).factor(), 0.8);
}

#[test]
fn zero_negative_and_non_finite_ratios_are_ignored() {
    let mut ratios = vec![0.8; MIN_CALIBRATION_SAMPLES];
    ratios.extend([0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
    let c = Calibration::from_ratios(ratios);
    assert_eq!(c.samples(), MIN_CALIBRATION_SAMPLES);
    assert_eq!(c.factor(), 0.8);
}

#[test]
fn club_median_uses_the_scoring_median() {
    let gains = [100, 200, 300, 400, 500, 600, 700, 800, 5000];
    assert_eq!(club_median(&club(&gains)), Some(500.0));
}

#[test]
fn club_median_needs_a_scorable_club() {
    assert_eq!(club_median(&club(&[100; 7])), None);
    assert_eq!(club_median(&club(&[0; 9])), None);
}
