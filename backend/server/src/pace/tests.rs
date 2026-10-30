use super::*;
use uma_stable_match_umamoe_client::CircleMember;

fn member(viewer_id: u64, month: i32, daily_fans: Vec<i64>) -> CircleMember {
    CircleMember {
        viewer_id,
        year: Some(2026),
        month: Some(month),
        daily_fans,
        ..Default::default()
    }
}

fn response(members: Vec<CircleMember>) -> CircleResponse {
    let mut response = CircleResponse {
        members,
        ..Default::default()
    };
    response.circle.circle_id = 77;
    response
}

fn found(pace: f64) -> Option<TrainerPace> {
    Some(TrainerPace {
        pace,
        circle_id: Some(77),
    })
}

fn pace(r: &CircleResponse, viewer_id: u64) -> Option<TrainerPace> {
    pace_from_response(r, viewer_id).into_result().ok()
}

#[test]
fn finds_the_trainers_pace() {
    let r = response(vec![
        member(1, 3, vec![10, 20, 30]),
        member(2, 3, vec![100, 400, 700]),
    ]);
    assert_eq!(pace(&r, 2), found(300.0));
}

#[test]
fn uses_the_latest_month() {
    let r = response(vec![
        member(2, 3, vec![100, 200, 300]),
        member(2, 2, vec![100, 900, 1700]),
    ]);
    assert_eq!(pace(&r, 2), found(100.0));
}

#[test]
fn missing_trainer_is_not_found() {
    assert_eq!(pace_from_response(&response(vec![]), 2), Outcome::NotFound);
    let r = response(vec![member(1, 3, vec![10, 20, 30])]);
    assert_eq!(pace_from_response(&r, 2), Outcome::NotFound);
}

#[test]
fn too_little_or_flat_data_is_not_enough() {
    for fans in [vec![10, 20, 0], vec![10, 10, 10]] {
        let outcome = pace_from_response(&response(vec![member(2, 3, fans)]), 2);
        assert!(matches!(
            outcome.into_result(),
            Err(PaceError::NotEnoughData)
        ));
    }
}

#[test]
fn errors_do_not_mention_the_trainer() {
    for error in [PaceError::NotFound, PaceError::NotEnoughData] {
        assert!(!error.to_string().chars().any(|c| c.is_ascii_digit()));
    }
}

#[test]
fn keeps_the_snapshot_and_its_month() {
    let r = response(vec![member(2, 3, vec![10, 20, 0])]);
    assert_eq!(
        pace_from_response(&r, 2),
        Outcome::Fans(Fans {
            daily_fans: vec![10, 20, 0],
            period: Some((2026, 3)),
            circle_id: Some(77),
        })
    );
}
