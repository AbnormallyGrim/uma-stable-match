use proptest::prelude::*;

use super::*;
use crate::calibration::MIN_CALIBRATION_SAMPLES;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn listed(circle_id: u64, raw_rate: f64) -> ListedClub {
    ListedClub {
        circle_id,
        member_count: 20,
        join_style: Some(JoinStyle::Approval),
        raw_rate,
    }
}

fn closed(circle_id: u64, raw_rate: f64) -> ListedClub {
    ListedClub {
        join_style: Some(JoinStyle::Closed),
        ..listed(circle_id, raw_rate)
    }
}

fn circle() -> Circle {
    Circle {
        circle_id: 7,
        member_count: Some(20),
        join_style: Some(2),
        monthly_point: Some(36_000_000),
        last_updated: Some("2026-03-19T15:00:00Z".into()),
        ..Default::default()
    }
}

#[test]
fn game_month_starts_on_the_2nd_in_jst() {
    let start = utc("2026-03-02T00:00:00+09:00");
    assert_eq!(game_month_start(utc("2026-03-20T00:00:00+09:00")), start);
    assert_eq!(game_month_start(utc("2026-03-31T23:59:59+09:00")), start);
    assert_eq!(game_month_start(utc("2026-04-01T00:00:00+09:00")), start);
    assert_eq!(game_month_start(utc("2026-04-01T23:59:59+09:00")), start);
    assert_eq!(
        game_month_start(utc("2026-04-02T00:00:00+09:00")),
        utc("2026-04-02T00:00:00+09:00")
    );
}

#[test]
fn game_month_uses_jst_not_utc() {
    assert_eq!(
        game_month_start(utc("2026-03-01T15:00:00Z")),
        utc("2026-03-02T00:00:00+09:00")
    );
    assert_eq!(
        game_month_start(utc("2026-03-01T14:59:59Z")),
        utc("2026-02-02T00:00:00+09:00")
    );
}

#[test]
fn january_1st_belongs_to_december() {
    assert_eq!(
        game_month_start(utc("2027-01-01T12:00:00+09:00")),
        utc("2026-12-02T00:00:00+09:00")
    );
}

#[test]
fn elapsed_days_count_from_the_month_start() {
    assert_eq!(elapsed_days(utc("2026-03-20T00:00:00+09:00")), 18.0);
    assert_eq!(elapsed_days(utc("2026-03-02T12:00:00+09:00")), 0.5);
    assert_eq!(elapsed_days(utc("2026-04-01T00:00:00+09:00")), 30.0);
}

#[test]
fn raw_rate_is_points_per_day_per_member() {
    let at = utc("2026-03-20T00:00:00+09:00");
    assert_eq!(raw_rate(36_000_000, 20, at), Some(100_000.0));
}

#[test]
fn raw_rate_needs_points_members_and_a_full_day() {
    let at = utc("2026-03-20T00:00:00+09:00");
    assert_eq!(raw_rate(0, 20, at), None);
    assert_eq!(raw_rate(-5, 20, at), None);
    assert_eq!(raw_rate(36_000_000, 0, at), None);
    assert_eq!(
        raw_rate(36_000_000, 20, utc("2026-03-02T00:00:00+09:00")),
        None
    );
    assert_eq!(
        raw_rate(36_000_000, 20, utc("2026-03-02T23:59:59+09:00")),
        None
    );
    assert!(raw_rate(36_000_000, 20, utc("2026-03-03T00:00:00+09:00")).is_some());
}

#[test]
fn listed_club_from_circle() {
    let club = ListedClub::from_circle(&circle()).unwrap();
    assert_eq!(club.circle_id, 7);
    assert_eq!(club.member_count, 20);
    assert_eq!(club.join_style, Some(JoinStyle::Approval));
    assert_eq!(club.raw_rate, 100_000.0);
}

#[test]
fn listed_club_filters() {
    let with = |f: fn(&mut Circle)| {
        let mut c = circle();
        f(&mut c);
        ListedClub::from_circle(&c)
    };
    assert!(with(|c| c.archived = Some(true)).is_none());
    assert!(with(|c| c.archived = Some(false)).is_some());
    assert!(with(|c| c.member_count = Some(30)).is_none());
    assert!(with(|c| c.member_count = Some(29)).is_some());
    assert!(with(|c| c.member_count = Some(8)).is_some());
    assert!(with(|c| c.member_count = Some(7)).is_none());
    assert!(with(|c| c.member_count = Some(-1)).is_none());
    assert!(with(|c| c.member_count = None).is_none());
    assert!(with(|c| c.monthly_point = None).is_none());
    assert!(with(|c| c.monthly_point = Some(0)).is_none());
    assert!(with(|c| c.last_updated = None).is_none());
    assert!(with(|c| c.last_updated = Some("yesterday".into())).is_none());
}

#[test]
fn closed_and_unknown_join_styles_stay_in_the_pool() {
    let mut c = circle();
    c.join_style = Some(3);
    assert_eq!(
        ListedClub::from_circle(&c).unwrap().join_style,
        Some(JoinStyle::Closed)
    );
    c.join_style = Some(9);
    assert_eq!(ListedClub::from_circle(&c).unwrap().join_style, None);
}

#[test]
fn pool_drops_duplicates_and_bad_rates() {
    let pool = ClubPool::new([
        listed(1, 100.0),
        listed(1, 200.0),
        listed(2, f64::NAN),
        listed(3, 0.0),
        listed(4, f64::INFINITY),
        listed(5, 50.0),
    ]);
    let ids: Vec<_> = pool.clubs().map(|c| c.circle_id).collect();
    assert_eq!(ids, vec![5, 1]);
    assert_eq!(pool.clubs().next().unwrap().raw_rate, 50.0);
}

fn pool() -> ClubPool {
    ClubPool::new([
        listed(1, 100.0),
        listed(2, 200.0),
        listed(3, 400.0),
        listed(4, 800.0),
        listed(5, 1600.0),
    ])
}

#[test]
fn shortlist_below_the_slowest_club() {
    assert_eq!(pool().shortlist(10.0, 3, false), vec![1, 2, 3]);
}

#[test]
fn shortlist_above_the_fastest_club() {
    assert_eq!(pool().shortlist(10_000.0, 3, false), vec![5, 4, 3]);
}

#[test]
fn shortlist_between_clubs_takes_the_nearest_in_log_space() {
    assert_eq!(pool().shortlist(300.0, 1, false), vec![3]);
    assert_eq!(pool().shortlist(250.0, 2, false), vec![2, 3]);
    assert_eq!(pool().shortlist(300.0, 3, false), vec![3, 2, 4]);
}

#[test]
fn shortlist_breaks_ties_by_circle_id() {
    let pool = ClubPool::new([
        listed(9, 200.0),
        listed(4, 200.0),
        listed(1, 3000.0),
        listed(6, 200.0),
    ]);
    assert_eq!(pool.shortlist(400.0, 1, false), vec![4]);
    assert_eq!(pool.shortlist(400.0, 2, false), vec![4, 6]);
    assert_eq!(pool.shortlist(400.0, 3, false), vec![4, 6, 9]);
    assert_eq!(pool.shortlist(100.0, 3, false), vec![4, 6, 9]);
}

#[test]
fn shortlist_skips_closed_clubs_unless_asked() {
    let pool = ClubPool::new([
        closed(1, 100.0),
        closed(2, 200.0),
        closed(3, 400.0),
        listed(4, 800.0),
        closed(5, 1600.0),
    ]);
    assert_eq!(pool.shortlist(400.0, 2, false), vec![4]);
    assert_eq!(pool.shortlist(300.0, 2, true), vec![3, 2]);
}

#[test]
fn shortlist_returns_everything_when_k_is_large() {
    let mut ids = pool().shortlist(400.0, 50, false);
    assert_eq!(ids[0], 3);
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2, 3, 4, 5]);
}

#[test]
fn shortlist_of_nothing() {
    assert!(pool().shortlist(400.0, 0, false).is_empty());
    assert!(pool().shortlist(0.0, 3, false).is_empty());
    assert!(pool().shortlist(-1.0, 3, false).is_empty());
    assert!(pool().shortlist(f64::NAN, 3, false).is_empty());
    assert!(ClubPool::default().shortlist(400.0, 3, false).is_empty());
}

#[test]
fn calibration_scales_the_estimates() {
    let mut pool = pool();
    let samples = std::iter::repeat_n(0.5, MIN_CALIBRATION_SAMPLES);
    pool.set_calibration(Calibration::from_ratios(samples));
    assert_eq!(pool.calibration().factor(), 0.5);
    assert_eq!(pool.estimated_median(&listed(1, 800.0)), 400.0);
    assert_eq!(pool.shortlist(400.0, 1, false), vec![4]);
}

fn brute_force(pool: &ClubPool, target: f64, k: usize, include_closed: bool) -> Vec<u64> {
    if k == 0 || !(target.is_finite() && target > 0.0) {
        return Vec::new();
    }
    let ln_target = target.ln() - pool.calibration().factor().ln();
    let mut all: Vec<(f64, u64)> = pool
        .entries
        .iter()
        .filter(|e| include_closed || e.club.join_style != Some(JoinStyle::Closed))
        .map(|e| (distance(e, ln_target), e.club.circle_id))
        .collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    all.into_iter().take(k).map(|(_, id)| id).collect()
}

fn arb_club() -> impl Strategy<Value = (f64, bool)> {
    // Some repeated rates, so ties are common.
    let rate = prop_oneof![
        3 => (8.0f64..20.0).prop_map(f64::exp),
        1 => prop::sample::select(vec![1e5, 5e5, 2e6]),
    ];
    (rate, prop::bool::weighted(0.3))
}

proptest! {
    #[test]
    fn shortlist_matches_brute_force(
        clubs in prop::collection::vec(arb_club(), 0..80),
        ln_target in 7.0f64..21.0,
        k in 0usize..30,
        include_closed: bool,
        factor in prop::option::of(0.2f64..2.0),
    ) {
        let mut pool = ClubPool::new(clubs.iter().enumerate().map(|(i, (rate, is_closed))| {
            let id = i as u64 + 1;
            if *is_closed { closed(id, *rate) } else { listed(id, *rate) }
        }));
        if let Some(f) = factor {
            pool.set_calibration(Calibration::from_ratios(
                std::iter::repeat_n(f, MIN_CALIBRATION_SAMPLES),
            ));
        }
        let target = ln_target.exp();
        prop_assert_eq!(
            pool.shortlist(target, k, include_closed),
            brute_force(&pool, target, k, include_closed)
        );
    }
}
