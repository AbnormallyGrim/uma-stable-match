use serde::Deserialize;
use uma_stable_match_scoring::{score_clubs, Club, ScoringOptions};
use uma_stable_match_umamoe_client::CircleResponse;

const CLUBS: &str = include_str!("fixtures/parity/clubs.json");
const EXPECTED: &str = include_str!("fixtures/parity/expected.json");

#[derive(Deserialize)]
struct Expected {
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    pace: f64,
    balance_weight: Option<f64>,
    results: Vec<ExpectedClub>,
}

#[derive(Deserialize)]
struct ExpectedClub {
    circle_id: u64,
    fit: f64,
    score: f64,
    median_per_day: f64,
    your_place: u32,
    club_size: u32,
    ranked_members: usize,
    balance: Option<ExpectedBalance>,
}

#[derive(Deserialize)]
struct ExpectedBalance {
    score: u8,
    gini: f64,
    passengers: usize,
    counted: usize,
    bottom_quarter_per_day: f64,
    top3_share: f64,
}

fn clubs() -> Vec<Club> {
    let responses: Vec<CircleResponse> = serde_json::from_str(CLUBS).unwrap();
    responses.into_iter().map(Club::from).collect()
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    let tolerance = 1e-9 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: got {actual}, prototype gave {expected}"
    );
}

#[test]
fn matches_prototype() {
    let clubs = clubs();
    let expected: Expected = serde_json::from_str(EXPECTED).unwrap();
    assert!(!expected.scenarios.is_empty());

    for scenario in &expected.scenarios {
        let options = ScoringOptions {
            balance_weight: scenario.balance_weight,
            include_closed: true,
        };
        let scored = score_clubs(scenario.pace, &clubs, &options).unwrap();
        let label = format!(
            "pace {} balance {:?}",
            scenario.pace, scenario.balance_weight
        );

        let ids: Vec<u64> = scored.iter().map(|s| s.club.circle_id).collect();
        let expected_ids: Vec<u64> = scenario.results.iter().map(|r| r.circle_id).collect();
        assert_eq!(ids, expected_ids, "{label}: clubs or order differ");

        for (actual, want) in scored.iter().zip(&scenario.results) {
            let what = format!("{label}, club {}", want.circle_id);
            assert_close(actual.fit, want.fit, &format!("{what} fit"));
            assert_close(actual.score, want.score, &format!("{what} score"));
            assert_close(
                actual.median_per_day,
                want.median_per_day,
                &format!("{what} median"),
            );
            assert_eq!(actual.your_place, want.your_place, "{what} place");
            assert_eq!(actual.club_size, want.club_size, "{what} size");
            assert_eq!(actual.ranked_members, want.ranked_members, "{what} ranked");

            match (&actual.balance, &want.balance) {
                (None, None) => {}
                (Some(a), Some(w)) => {
                    assert_eq!(a.score, w.score, "{what} balance score");
                    assert_eq!(a.passengers, w.passengers, "{what} passengers");
                    assert_eq!(a.counted, w.counted, "{what} counted");
                    assert_close(a.gini, w.gini, &format!("{what} gini"));
                    assert_close(
                        a.bottom_quarter_per_day,
                        w.bottom_quarter_per_day,
                        &format!("{what} bottom quarter"),
                    );
                    assert_close(a.top3_share, w.top3_share, &format!("{what} top 3 share"));
                }
                _ => panic!("{what}: balance presence differs"),
            }
        }
    }
}

#[test]
fn closed_clubs_are_excluded_by_default() {
    let clubs = clubs();
    let scored = score_clubs(7_600_000.0, &clubs, &ScoringOptions::default()).unwrap();
    assert!(!scored.is_empty());
    assert!(scored.iter().all(|s| s.club.circle_id != 900106));
}
