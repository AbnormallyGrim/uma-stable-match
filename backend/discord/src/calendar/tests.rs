use super::*;

fn at(year: i64, month: i64, day: i64, hour: u64) -> u64 {
    days_from_civil(year, month, day) as u64 * DAY + hour * 3_600
}

#[test]
fn formats_unix_seconds_as_iso() {
    assert_eq!(iso(0), "1970-01-01T00:00:00Z");
    assert_eq!(
        iso(at(2026, 11, 6, 9) + 30 * 60 + 5),
        "2026-11-06T09:30:05Z"
    );
    assert_eq!(iso(at(2028, 2, 29, 23)), "2028-02-29T23:00:00Z");
}

#[test]
fn adds_a_calendar_month() {
    assert_eq!(add_month(at(2026, 11, 6, 9)), at(2026, 12, 6, 9));
    assert_eq!(add_month(at(2026, 12, 15, 1)), at(2027, 1, 15, 1));
}

#[test]
fn clamps_to_the_last_day_of_a_shorter_month() {
    assert_eq!(add_month(at(2026, 1, 31, 12)), at(2026, 2, 28, 12));
    assert_eq!(add_month(at(2028, 1, 31, 12)), at(2028, 2, 29, 12));
    assert_eq!(add_month(at(2026, 3, 31, 0)), at(2026, 4, 30, 0));
}
