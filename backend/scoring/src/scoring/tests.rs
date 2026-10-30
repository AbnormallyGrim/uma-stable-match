use super::*;

const M: i64 = 1_000_000;

/// A member gaining `per_day` fans each day for `days` days.
fn member(id: u64, per_day: i64, days: usize) -> MemberSnapshot {
    let start = 100 * M;
    MemberSnapshot {
        viewer_id: id,
        year: Some(2026),
        month: Some(10),
        daily_fans: (0..days).map(|d| start + per_day * d as i64).collect(),
        prior: None,
    }
}

/// A club whose members gain the given daily amounts over 10 days.
fn club(name: &str, daily_gains: &[i64]) -> Club {
    Club {
        circle_id: 1,
        name: name.into(),
        member_count: Some(daily_gains.len() as u32),
        join_style: Some(JoinStyle::Approval),
        members: daily_gains
            .iter()
            .enumerate()
            .map(|(i, g)| member(i as u64 + 1, *g, 10))
            .collect(),
        ..Default::default()
    }
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn place_and_fit_follow_the_formula() {
    // 10 members at 1M..10M per day; you gain 7.5M/day.
    let gains: Vec<i64> = (1..=10).map(|g| g * M).collect();
    let c = club("Test", &gains);
    let s = score_club(7.5e6, &c, &ScoringOptions::default())
        .unwrap()
        .unwrap();

    assert_eq!(s.club_size, 11);
    assert_eq!(s.your_place, 4); // 8M, 9M, 10M are faster
    assert!(approx(s.median_per_day, 5.5e6));
    let expected = (4.0 / 11.0 - 0.4f64).abs() + 0.3 * (5.5e6 / 7.5e6 - 0.85f64).abs();
    assert!(approx(s.fit, expected));
    assert!(approx(s.score, s.fit));
    assert!(s.balance.is_none());
}

#[test]
fn club_size_is_capped_at_thirty() {
    let mut c = club("Full", &[5 * M; 12]);
    c.member_count = Some(30);
    let s = score_club(1e6, &c, &ScoringOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(s.club_size, CLUB_CAP);
    assert_eq!(s.your_place, 13);
}

#[test]
fn missing_member_count_uses_current_members() {
    let mut c = club("Unknown size", &[5 * M; 9]);
    c.member_count = None;
    let s = score_club(6e6, &c, &ScoringOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(s.club_size, 10);
    assert_eq!(s.your_place, 1);
}

#[test]
fn new_members_count_towards_size_but_not_place() {
    let mut c = club("Recruiting", &[5 * M; 8]);
    // Two members who joined on the last day: current, but unrankable.
    for id in [100, 101] {
        let mut fans = vec![0; 10];
        fans[9] = 50 * M;
        c.members.push(MemberSnapshot {
            viewer_id: id,
            year: Some(2026),
            month: Some(10),
            daily_fans: fans,
            prior: None,
        });
    }
    c.member_count = Some(10);
    let s = score_club(4e6, &c, &ScoringOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(s.ranked_members, 8);
    assert_eq!(s.club_size, 11);
    assert_eq!(s.your_place, 9);
}

#[test]
fn departed_members_are_ignored() {
    let mut c = club("Turnover", &[5 * M; 8]);
    let mut left = member(99, 50 * M, 10);
    for v in left.daily_fans.iter_mut().skip(5) {
        *v = 0;
    }
    c.members.push(left);
    let s = score_club(6e6, &c, &ScoringOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(s.ranked_members, 8);
    assert_eq!(s.your_place, 1);
}

#[test]
fn small_clubs_are_left_out() {
    let c = club("Tiny", &[5 * M; 7]);
    assert!(score_club(5e6, &c, &ScoringOptions::default())
        .unwrap()
        .is_none());
}

#[test]
fn closed_clubs_need_include_closed() {
    let mut c = club("Closed", &[5 * M; 10]);
    c.join_style = Some(JoinStyle::Closed);
    let default = ScoringOptions::default();
    assert!(score_club(5e6, &c, &default).unwrap().is_none());
    let include = ScoringOptions {
        include_closed: true,
        ..Default::default()
    };
    assert!(score_club(5e6, &c, &include).unwrap().is_some());
}

#[test]
fn balance_of_an_even_club() {
    let c = club("Even", &[5 * M; 10]);
    let b = club_balance(&c.members).unwrap();
    assert_eq!(b.score, 100);
    assert_eq!(b.passengers, 0);
    assert_eq!(b.counted, 10);
    assert!(approx(b.gini, 0.0));
    assert!(approx(b.top3_share, 0.3));
    assert!(approx(b.bottom_quarter_per_day, 5e6));
}

#[test]
fn passengers_use_the_higher_threshold() {
    // Median 8M, so the floor is max(1M, 2M) = 2M.
    let mut gains = vec![8 * M; 8];
    gains.extend([1_500_000, 500_000]);
    let b = club_balance(&club("Mixed", &gains).members).unwrap();
    assert_eq!(b.passengers, 2);

    // Median 2M, so the floor is the 1M minimum.
    let mut gains = vec![2 * M; 8];
    gains.extend([900_000, 1_100_000]);
    let b = club_balance(&club("Low", &gains).members).unwrap();
    assert_eq!(b.passengers, 1);
}

#[test]
fn balance_formula() {
    let gains: Vec<i64> = (1..=10).map(|g| g * M).collect();
    let b = club_balance(&club("Spread", &gains).members).unwrap();
    let rates: Vec<f64> = (1..=10).map(|g| g as f64 * 1e6).collect();
    let g = gini(&rates);
    // Floor = max(1M, 0.25 × 5.5M) = 1.375M: only the 1M member is a passenger.
    assert_eq!(b.passengers, 1);
    let expected = (100.0 * (0.5 * (1.0 - g) + 0.5 * (1.0 - 0.1))).round();
    assert_eq!(f64::from(b.score), expected);
    assert!(approx(b.top3_share, 27.0 / 55.0));
    assert!(approx(b.bottom_quarter_per_day, 1.5e6)); // median of 2M, 1M
}

#[test]
fn balance_needs_enough_members() {
    assert!(club_balance(&club("Few", &[5 * M; 7]).members).is_none());
}

#[test]
fn balance_weight_adjusts_score() {
    let mut gains = vec![8 * M; 7];
    gains.extend([0, 0, 0]);
    let c = club("Passengers", &gains);
    let options = ScoringOptions {
        balance_weight: Some(0.8),
        ..Default::default()
    };
    let s = score_club(7e6, &c, &options).unwrap().unwrap();
    let b = s.balance.unwrap();
    assert!(approx(
        s.score,
        s.fit + 0.8 * (1.0 - f64::from(b.score) / 100.0)
    ));
    assert!(s.score > s.fit);
}

#[test]
fn ranking_is_best_first_and_balance_can_reorder() {
    // Same fit, but "Carried" has passengers.
    let even = club("Even", &[6 * M; 10]);
    let mut carried_gains = vec![10 * M; 5];
    carried_gains.extend([2 * M; 5]);
    let carried = club("Carried", &carried_gains);
    let clubs = vec![carried, even];

    let plain = score_clubs(7e6, &clubs, &ScoringOptions::default()).unwrap();
    assert!(plain.windows(2).all(|w| w[0].score <= w[1].score));

    let balanced = score_clubs(
        7e6,
        &clubs,
        &ScoringOptions {
            balance_weight: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(balanced[0].club.name, "Even");
    assert!(balanced.windows(2).all(|w| w[0].score <= w[1].score));
}

#[test]
fn invalid_inputs_are_rejected() {
    let clubs = vec![club("Any", &[5 * M; 10])];
    let opts = ScoringOptions::default();
    assert_eq!(
        score_clubs(0.0, &clubs, &opts).unwrap_err(),
        ScoringError::InvalidPace(0.0)
    );
    assert!(score_clubs(f64::NAN, &clubs, &opts).is_err());
    let bad = ScoringOptions {
        balance_weight: Some(1.5),
        ..Default::default()
    };
    assert_eq!(
        score_clubs(5e6, &clubs, &bad).unwrap_err(),
        ScoringError::InvalidBalanceWeight(1.5)
    );
}

/// `member` with a prior.
fn with_prior(mut m: MemberSnapshot, prior: f64) -> MemberSnapshot {
    m.prior = Some(prior);
    m
}

#[test]
fn blended_median_uses_priors_for_new_members() {
    // Eight members with 2 days of data, which daily_pace can't rank yet.
    let new = |id: u64| member(id, 2 * M, 2);
    let members: Vec<MemberSnapshot> = (1..=8)
        .map(|id| with_prior(new(id), if id <= 4 { 4.0 } else { 6.0 } * M as f64))
        .collect();
    // Each blend is half of 2M and half of the prior.
    assert_eq!(blended_median(&members, 1.0), Some(3.5 * M as f64));
    let without: Vec<MemberSnapshot> = (1..=8).map(new).collect();
    assert_eq!(blended_median(&without, 1.0), None);
}

#[test]
fn blended_median_leaves_out_members_who_left() {
    let mut members: Vec<MemberSnapshot> = (1..=8)
        .map(|id| with_prior(member(id, M, 3), M as f64))
        .collect();
    let mut left = with_prior(member(9, 50 * M, 3), 50.0 * M as f64);
    left.daily_fans[2] = 0;
    members.push(left);
    assert_eq!(blended_median(&members, 1.0), Some(M as f64));
}

#[test]
fn blended_median_needs_enough_members() {
    let members: Vec<MemberSnapshot> = (1..=7)
        .map(|id| with_prior(member(id, M, 5), M as f64))
        .collect();
    assert_eq!(blended_median(&members, 1.0), None);
}

#[test]
fn blended_median_counts_falling_paces_as_zero() {
    let members: Vec<MemberSnapshot> = (1..=8)
        .map(|id| with_prior(member(id, 0, 3), -5.0 * M as f64))
        .collect();
    assert_eq!(blended_median(&members, 1.0), Some(0.0));
}

#[test]
fn prior_k_scores_new_members_from_their_priors() {
    let mut club = club("New", &[]);
    club.members = (1..=10)
        .map(|id| with_prior(member(id, 2 * M, 2), 4.0 * M as f64))
        .collect();
    club.member_count = Some(10);
    let options = ScoringOptions::default();
    assert!(score_club(3.0 * M as f64, &club, &options)
        .unwrap()
        .is_none());

    let scored = score_club(3.0 * M as f64, &club, &options.with_prior_k(1.0))
        .unwrap()
        .unwrap();
    assert_eq!(scored.median_per_day, 3.0 * M as f64);
    assert_eq!(scored.ranked_members, 10);
}

#[test]
fn prior_k_without_priors_scores_as_before() {
    let club = club(
        "Steady",
        &[M, 2 * M, 3 * M, 4 * M, 5 * M, 6 * M, 7 * M, 8 * M],
    );
    let plain = score_club(4.0 * M as f64, &club, &ScoringOptions::default()).unwrap();
    let blended = score_club(
        4.0 * M as f64,
        &club,
        &ScoringOptions::default().with_prior_k(1.0),
    )
    .unwrap();
    assert_eq!(plain.map(|s| s.score), blended.map(|s| s.score));
}

#[test]
fn rejects_an_invalid_prior_k() {
    let club = club("Any", &[M; 8]);
    for k in [-1.0, f64::NAN] {
        let options = ScoringOptions::default().with_prior_k(k);
        assert!(matches!(
            score_club(M as f64, &club, &options),
            Err(ScoringError::InvalidPriorK(_))
        ));
    }
}

#[test]
fn prior_k_counts_new_members_in_balance() {
    let mut club = club("New", &[]);
    club.members = (1..=10)
        .map(|id| with_prior(member(id, 2 * M, 2), 4.0 * M as f64))
        .collect();
    club.member_count = Some(10);
    let options = ScoringOptions::default().with_balance_weight(0.5);
    assert!(score_club(3.0 * M as f64, &club, &options)
        .unwrap()
        .is_none());

    let scored = score_club(3.0 * M as f64, &club, &options.with_prior_k(1.0))
        .unwrap()
        .unwrap();
    assert_eq!(scored.balance.unwrap().counted, 10);
}
