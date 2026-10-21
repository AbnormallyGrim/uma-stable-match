use super::*;
use chrono::TimeZone;

fn at(year: i32, month: u32, day: u32, hour: u32) -> SystemTime {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0)
        .unwrap()
        .into()
}

const INTERVAL: Duration = Duration::from_secs(3 * 3600);

#[test]
fn a_recent_scan_in_the_same_month_is_resumed() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 9));
    assert!(progress.is_current(at(2026, 3, 18, 10), INTERVAL));
}

#[test]
fn an_old_scan_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 9));
    assert!(!progress.is_current(at(2026, 3, 18, 12), INTERVAL));
}

#[test]
fn a_scan_from_another_game_month_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 3, 14));
    assert!(!progress.is_current(at(2026, 3, 3, 16), INTERVAL));
}

#[test]
fn a_scan_from_the_future_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 10));
    assert!(!progress.is_current(at(2026, 3, 18, 9), INTERVAL));
}
