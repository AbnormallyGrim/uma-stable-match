use super::*;
use uma_stable_match_umamoe_client::{ApiErrorBody, StatusCode};

fn circle(monthly_rank: Option<i32>, member_count: Option<i32>) -> Circle {
    Circle {
        circle_id: 1,
        monthly_rank,
        member_count,
        ..Default::default()
    }
}

#[test]
fn candidate_needs_rank_in_window() {
    let config = ScanConfig::default();
    assert!(is_candidate(&circle(Some(80), Some(20)), &config));
    assert!(is_candidate(&circle(Some(900), Some(20)), &config));
    assert!(!is_candidate(&circle(Some(79), Some(20)), &config));
    assert!(!is_candidate(&circle(Some(901), Some(20)), &config));
    assert!(!is_candidate(&circle(None, Some(20)), &config));
    assert!(!is_candidate(&circle(Some(-1), Some(20)), &config));
}

#[test]
fn candidate_needs_open_spots() {
    let config = ScanConfig::default();
    assert!(is_candidate(&circle(Some(100), Some(29)), &config));
    assert!(!is_candidate(&circle(Some(100), Some(30)), &config));
    assert!(!is_candidate(&circle(Some(100), None), &config));
    assert!(!is_candidate(&circle(Some(100), Some(0)), &config));
}

#[test]
fn candidate_needs_enough_members_to_score() {
    let config = ScanConfig::default();
    assert!(is_candidate(&circle(Some(100), Some(8)), &config));
    assert!(!is_candidate(&circle(Some(100), Some(7)), &config));
    assert!(!is_candidate(&circle(Some(100), Some(1)), &config));
}

#[test]
fn candidate_is_not_archived() {
    let config = ScanConfig::default();
    let mut c = circle(Some(100), Some(20));
    c.archived = Some(true);
    assert!(!is_candidate(&c, &config));
    c.archived = Some(false);
    assert!(is_candidate(&c, &config));
}

#[test]
fn closed_clubs_are_candidates() {
    let config = ScanConfig::default();
    let mut c = circle(Some(100), Some(20));
    c.join_style = Some(3);
    assert!(is_candidate(&c, &config));
}

#[test]
fn public_message_hides_the_api_error_body() {
    let error = ScanError::List(Error::Status {
        status: StatusCode::SERVICE_UNAVAILABLE,
        body: Some(ApiErrorBody {
            error: "made-up internal detail".into(),
            status: 503,
            details: Some("made-up host name".into()),
        }),
    });
    assert_eq!(
        error.public_message(),
        "listing clubs failed: uma.moe returned 503 Service Unavailable"
    );
}

#[test]
fn public_message_for_decode_errors() {
    let decode = serde_json::from_str::<u32>("not json").unwrap_err();
    let error = ScanError::List(Error::Decode(decode));
    assert_eq!(
        error.public_message(),
        "listing clubs failed: unexpected response from uma.moe"
    );
}
