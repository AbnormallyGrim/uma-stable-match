use super::*;

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().into()
}

fn minutes(n: u64) -> Duration {
    Duration::from_secs(n * 60)
}

#[test]
fn defaults_follow_the_daily_update() {
    let config = RefreshConfig::default();
    assert_eq!(config.every, None);
    assert_eq!(config.margin, minutes(30));
    assert_eq!(config.recheck, minutes(60));
    assert_eq!(config.retry, minutes(5));
    assert_eq!(config.fast_ranks, 5_000);
}

#[test]
fn next_refresh_is_a_day_after_the_update_plus_the_margin() {
    let config = RefreshConfig::default();
    let updated = at("2026-03-19T15:00:00Z");
    let now = at("2026-03-19T18:00:00Z");
    assert_eq!(
        config.next_refresh(Some(updated), now),
        at("2026-03-20T15:30:00Z")
    );
}

#[test]
fn rechecks_later_when_uma_moe_has_not_updated_yet() {
    let config = RefreshConfig::default();
    let updated = at("2026-03-19T15:00:00Z");
    let now = at("2026-03-20T15:31:00Z");
    assert_eq!(
        config.next_refresh(Some(updated), now),
        at("2026-03-20T16:31:00Z")
    );
    assert_eq!(config.next_refresh(None, now), at("2026-03-20T16:31:00Z"));
}

#[test]
fn a_fixed_interval_overrides_the_daily_update() {
    let config = RefreshConfig {
        every: Some(minutes(90)),
        ..RefreshConfig::default()
    };
    let updated = at("2026-03-19T15:00:00Z");
    let now = at("2026-03-19T18:00:00Z");
    assert_eq!(
        config.next_refresh(Some(updated), now),
        at("2026-03-19T19:30:00Z")
    );
}

#[test]
fn orders_by_due_time_then_rank() {
    let now = at("2026-03-19T18:00:00Z");
    let mut schedule = Schedule::default();
    schedule.replace([(1, 30), (2, 10), (3, 20)], |_| true, now);
    assert_eq!(schedule.next(), Some((now, 10, 2)));

    schedule.reschedule(2, now + minutes(5));
    assert_eq!(schedule.next(), Some((now, 20, 3)));
    schedule.reschedule(3, now + minutes(1));
    schedule.reschedule(1, now + minutes(1));
    assert_eq!(schedule.next(), Some((now + minutes(1), 20, 3)));
    assert_eq!(schedule.len(), 3);
}

#[test]
fn a_listing_adds_new_clubs_due_now_and_drops_clubs_that_left() {
    let now = at("2026-03-19T18:00:00Z");
    let later = now + minutes(60);
    let mut schedule = Schedule::default();
    schedule.replace([(1, 10), (2, 20)], |_| true, now);
    schedule.reschedule(1, later);
    schedule.reschedule(2, later);
    assert!(schedule.insert_new(3, 30, later));

    schedule.replace([(1, 15), (4, 40)], |id| id != 2, now + minutes(10));
    assert_eq!(schedule.due_at(1), Some(later));
    assert_eq!(schedule.due_at(2), None);
    assert_eq!(schedule.due_at(3), Some(later));
    assert_eq!(schedule.due_at(4), Some(now + minutes(10)));
    assert_eq!(schedule.next(), Some((now + minutes(10), 40, 4)));
    schedule.reschedule(4, later);
    assert_eq!(schedule.next(), Some((later, 15, 1)));
}

#[test]
fn loaded_clubs_join_once() {
    let now = at("2026-03-19T18:00:00Z");
    let mut schedule = Schedule::default();
    assert!(schedule.insert_new(1, 10, now));
    assert!(!schedule.insert_new(1, 10, now + minutes(5)));
    assert_eq!(schedule.due_at(1), Some(now));
}

#[test]
fn rescheduling_a_dropped_club_does_nothing() {
    let now = at("2026-03-19T18:00:00Z");
    let mut schedule = Schedule::default();
    schedule.reschedule(1, now);
    assert_eq!(schedule.len(), 0);
    assert_eq!(schedule.next(), None);
}
