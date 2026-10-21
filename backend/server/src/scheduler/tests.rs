use super::*;
use chrono::TimeZone;

fn at(year: i32, month: u32, day: u32, hour: u32) -> SystemTime {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0)
        .unwrap()
        .into()
}

const INTERVAL: Duration = Duration::from_secs(3 * 3600);

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().into()
}

#[test]
fn lists_again_after_uma_moes_next_update() {
    let updated = utc("2026-03-19T15:00:00Z");
    assert_eq!(
        until_next_update(Some(updated), utc("2026-03-19T16:00:00Z")),
        Duration::from_secs(23 * 3600 + 30 * 60)
    );
    assert_eq!(
        until_next_update(Some(updated), utc("2026-03-20T15:29:00Z")),
        Duration::from_secs(60)
    );
}

#[test]
fn lists_again_an_hour_later_when_uma_moe_has_not_updated() {
    let updated = utc("2026-03-19T15:00:00Z");
    assert_eq!(
        until_next_update(Some(updated), utc("2026-03-20T15:30:00Z")),
        Duration::from_secs(3600)
    );
    assert_eq!(
        until_next_update(None, utc("2026-03-20T15:30:00Z")),
        Duration::from_secs(3600)
    );
}

#[test]
fn a_recent_scan_in_the_same_month_is_resumed() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 9), at(2026, 3, 18, 9));
    assert!(progress.is_current(at(2026, 3, 18, 10), INTERVAL));
}

#[test]
fn an_old_scan_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 9), at(2026, 3, 18, 9));
    assert!(!progress.is_current(at(2026, 3, 18, 12), INTERVAL));
}

#[test]
fn a_scan_from_another_game_month_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 3, 14), at(2026, 3, 3, 14));
    assert!(!progress.is_current(at(2026, 3, 3, 16), INTERVAL));
}

#[test]
fn a_scan_from_the_future_is_listed_again() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 10), at(2026, 3, 18, 10));
    assert!(!progress.is_current(at(2026, 3, 18, 9), INTERVAL));
}

#[test]
fn a_saved_scan_keeps_its_original_start() {
    let progress = InProgress::new(Vec::new(), at(2026, 3, 18, 8), at(2026, 3, 18, 9));
    assert_eq!(progress.started(), at(2026, 3, 18, 8));
}

#[test]
fn a_saved_scan_without_a_start_uses_when_it_was_listed() {
    let saved: InProgress =
        serde_json::from_str(r#"{ "listed_at_ms": 1773824400000, "rows": [] }"#).unwrap();
    assert_eq!(saved.started(), at(2026, 3, 18, 9));
}
