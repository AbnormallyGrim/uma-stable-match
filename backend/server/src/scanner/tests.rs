use super::*;

fn circle(monthly_rank: Option<i32>, member_count: Option<i32>) -> Circle {
    Circle {
        circle_id: 1,
        monthly_rank,
        member_count,
        ..Default::default()
    }
}

#[test]
fn candidate_needs_rank_in_window() {
    let config = ScanConfig::default();
    assert!(is_candidate(&circle(Some(80), Some(20)), &config));
    assert!(is_candidate(&circle(Some(900), Some(20)), &config));
    assert!(!is_candidate(&circle(Some(79), Some(20)), &config));
    assert!(!is_candidate(&circle(Some(901), Some(20)), &config));
    assert!(!is_candidate(&circle(None, Some(20)), &config));
    assert!(!is_candidate(&circle(Some(-1), Some(20)), &config));
}

#[test]
fn candidate_needs_open_spots() {
    let config = ScanConfig::default();
    assert!(is_candidate(&circle(Some(100), Some(29)), &config));
    assert!(!is_candidate(&circle(Some(100), Some(30)), &config));
    assert!(!is_candidate(&circle(Some(100), None), &config));
    assert!(!is_candidate(&circle(Some(100), Some(0)), &config));
}

#[test]
fn climb_is_last_month_minus_this_month() {
    let c = |now, last| Circle {
        monthly_rank: now,
        last_month_rank: last,
        ..Default::default()
    };
    assert_eq!(climb(&c(Some(100), Some(150))), 50);
    assert_eq!(climb(&c(Some(150), Some(100))), -50);
    assert_eq!(climb(&c(Some(100), None)), 1_000_000_000 - 100);
    assert_eq!(climb(&c(None, Some(100))), 100);
}
