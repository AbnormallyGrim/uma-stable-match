//! Shortlist recall against exhaustive scoring on synthetic pools. All data is made up.

use std::collections::HashSet;

use uma_stable_match_scoring::{score_clubs, Club, JoinStyle, MemberSnapshot, ScoringOptions};
use uma_stable_match_server::{
    club_median, ClubPool, ListedClub, DEFAULT_SHORTLIST_SIZE, TARGET_MEDIAN_RATIO,
};
use uma_stable_match_umamoe_client::Circle;

const CLUBS: u64 = 500;
const DAYS: i64 = 18;
const SNAPSHOT: &str = "2026-03-19T15:00:00Z";
const PREFETCH: usize = 80;
const TOP: usize = 10;

struct Synthetic {
    clubs: Vec<Club>,
    listed: Vec<ListedClub>,
}

fn normal(rng: &mut fastrand::Rng) -> f64 {
    let u1 = rng.f64().max(f64::MIN_POSITIVE);
    let u2 = rng.f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn synthetic(seed: u64) -> Synthetic {
    let mut rng = fastrand::Rng::with_seed(seed);
    let (low, high) = (300_000f64.ln(), 9_000_000f64.ln());
    let mut clubs = Vec::new();
    let mut listed = Vec::new();
    for id in 1..=CLUBS {
        let size = rng.u32(8..30);
        let ln_median = low + rng.f64() * (high - low);
        let sigma = 0.2 + rng.f64() * 0.6;
        let mut gains: Vec<i64> = (0..size)
            .map(|_| (ln_median + sigma * normal(&mut rng)).exp() as i64)
            .collect();
        if rng.f64() < 0.1 {
            for gain in gains.iter_mut().take(rng.usize(1..4)) {
                *gain *= rng.i64(3..7);
            }
        }
        let join_style = if rng.f64() < 0.15 { 3 } else { 2 };

        let mut club = Club::new(id, format!("Synthetic Club {id}"));
        club.member_count = Some(size);
        club.join_style = JoinStyle::from_code(join_style);
        club.members = gains
            .iter()
            .enumerate()
            .map(|(i, gain)| {
                let start = rng.i64(10_000_000..500_000_000);
                let fans = (0..=DAYS).map(|day| start + gain * day).collect();
                let mut member = MemberSnapshot::new(id * 100 + i as u64, fans);
                member.year = Some(2026);
                member.month = Some(3);
                member
            })
            .collect();
        clubs.push(club);

        let circle = Circle {
            circle_id: id,
            member_count: Some(size as i32),
            join_style: Some(join_style as i32),
            monthly_point: Some(gains.iter().sum::<i64>() * DAYS),
            last_updated: Some(SNAPSHOT.into()),
            ..Default::default()
        };
        listed.push(ListedClub::from_circle(&circle).expect("synthetic club is eligible"));
    }
    Synthetic { clubs, listed }
}

fn calibrated_pool(data: &Synthetic) -> ClubPool {
    let mut pool = ClubPool::new(data.listed.clone());
    let sorted: Vec<u64> = pool.clubs().map(|l| l.circle_id).collect();
    let step = sorted.len() as f64 / PREFETCH as f64;
    for i in 0..PREFETCH {
        let id = sorted[(i as f64 * step) as usize];
        if let Some(median) = club_median(&data.clubs[(id - 1) as usize]) {
            pool.set_exact_median(id, median);
        }
    }
    pool
}

fn top_ids(pace: f64, clubs: &[Club], options: &ScoringOptions) -> Vec<u64> {
    score_clubs(pace, clubs, options)
        .unwrap()
        .iter()
        .take(TOP)
        .map(|s| s.club.circle_id)
        .collect()
}

fn recalls(data: &Synthetic, pool: &ClubPool, options: &ScoringOptions) -> Vec<f64> {
    let (low, high) = (400_000f64.ln(), 8_000_000f64.ln());
    (0..40)
        .map(|i| (low + (high - low) * f64::from(i) / 39.0).exp())
        .map(|pace| {
            let exhaustive = top_ids(pace, &data.clubs, options);
            let shortlist: HashSet<u64> = pool
                .shortlist(
                    TARGET_MEDIAN_RATIO * pace,
                    DEFAULT_SHORTLIST_SIZE,
                    options.include_closed,
                )
                .into_iter()
                .collect();
            let candidates: Vec<Club> = data
                .clubs
                .iter()
                .filter(|c| shortlist.contains(&c.circle_id))
                .cloned()
                .collect();
            let two_stage: HashSet<u64> = top_ids(pace, &candidates, options).into_iter().collect();
            let hits = exhaustive
                .iter()
                .filter(|id| two_stage.contains(id))
                .count();
            hits as f64 / exhaustive.len().max(1) as f64
        })
        .collect()
}

fn assert_recall(options: ScoringOptions) {
    for seed in [87, 88, 89] {
        let data = synthetic(seed);
        let pool = calibrated_pool(&data);
        assert!(pool.calibration().is_calibrated());
        let recalls = recalls(&data, &pool, &options);
        let mean = recalls.iter().sum::<f64>() / recalls.len() as f64;
        let worst = recalls.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(mean >= 0.95, "seed {seed}: mean recall@10 {mean:.3}");
        assert!(worst >= 0.8, "seed {seed}: worst recall@10 {worst:.3}");
    }
}

#[test]
fn shortlist_recall_at_default_settings() {
    assert_recall(ScoringOptions::default());
}

#[test]
fn shortlist_recall_with_closed_clubs() {
    assert_recall(ScoringOptions::default().with_include_closed(true));
}

#[test]
fn shortlist_recall_with_balance_scoring() {
    assert_recall(ScoringOptions::default().with_balance_weight(0.5));
}
