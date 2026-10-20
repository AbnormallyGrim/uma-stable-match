use super::*;
use uma_stable_match_umamoe_client::{ApiErrorBody, StatusCode};

fn circle(monthly_rank: Option<i32>) -> Circle {
    Circle {
        circle_id: 1,
        monthly_rank,
        ..Default::default()
    }
}

fn listed(circle_id: u64, raw_rate: f64) -> ListedClub {
    ListedClub {
        circle_id,
        member_count: 20,
        join_style: None,
        raw_rate,
    }
}

#[test]
fn window_is_the_rank_range() {
    let config = ScanConfig::default();
    assert!(in_window(&circle(Some(80)), &config));
    assert!(in_window(&circle(Some(900)), &config));
    assert!(!in_window(&circle(Some(79)), &config));
    assert!(!in_window(&circle(Some(901)), &config));
    assert!(!in_window(&circle(None), &config));
    assert!(!in_window(&circle(Some(-1)), &config));
}

#[test]
fn prefetch_takes_everything_that_fits() {
    let pool = ClubPool::new((1..=3).map(|id| listed(id, 100.0 * id as f64)));
    assert_eq!(prefetch_ids(&pool, 3), vec![1, 2, 3]);
    assert_eq!(prefetch_ids(&pool, 10), vec![1, 2, 3]);
}

#[test]
fn prefetch_spreads_across_the_estimates() {
    let pool = ClubPool::new((1..=10).map(|id| listed(id, 100.0 * id as f64)));
    assert_eq!(prefetch_ids(&pool, 5), vec![1, 3, 5, 7, 9]);
    assert_eq!(prefetch_ids(&pool, 3), vec![1, 4, 7]);
    assert!(prefetch_ids(&pool, 0).is_empty());
}

#[test]
fn month_labels_are_year_and_month() {
    assert_eq!(month_label((2026, 3)), "2026-03");
    assert_eq!(month_label((2026, 12)), "2026-12");
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
