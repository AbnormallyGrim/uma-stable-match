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
    CircleResponse {
        members,
        ..Default::default()
    }
}

#[test]
fn finds_the_trainers_pace() {
    let r = response(vec![
        member(1, 3, vec![10, 20, 30]),
        member(2, 3, vec![100, 400, 700]),
    ]);
    assert_eq!(pace_from_response(&r, 2), Outcome::Pace(300.0));
}

#[test]
fn uses_the_latest_month() {
    let r = response(vec![
        member(2, 3, vec![100, 200, 300]),
        member(2, 2, vec![100, 900, 1700]),
    ]);
    assert_eq!(pace_from_response(&r, 2), Outcome::Pace(100.0));
}

#[test]
fn missing_trainer_is_not_found() {
    assert_eq!(pace_from_response(&response(vec![]), 2), Outcome::NotFound);
    let r = response(vec![member(1, 3, vec![10, 20, 30])]);
    assert_eq!(pace_from_response(&r, 2), Outcome::NotFound);
}

#[test]
fn too_little_or_flat_data_is_not_enough() {
    let r = response(vec![member(2, 3, vec![10, 20, 0])]);
    assert_eq!(pace_from_response(&r, 2), Outcome::NotEnoughData);
    let r = response(vec![member(2, 3, vec![10, 10, 10])]);
    assert_eq!(pace_from_response(&r, 2), Outcome::NotEnoughData);
}

#[test]
fn errors_do_not_mention_the_trainer() {
    for error in [PaceError::NotFound, PaceError::NotEnoughData] {
        assert!(!error.to_string().chars().any(|c| c.is_ascii_digit()));
    }
}
