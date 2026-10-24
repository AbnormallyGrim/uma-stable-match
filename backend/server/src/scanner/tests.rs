use super::*;
use uma_stable_match_scoring::CLUB_CAP;
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

fn ranked(circle_id: u64, monthly_rank: Option<i32>) -> Circle {
    Circle {
        circle_id,
        monthly_rank,
        ..Default::default()
    }
}

#[test]
fn prefetch_takes_everything_that_fits() {
    let pool = ClubPool::new((1..=3).map(|id| listed(id, 100.0 * id as f64)));
    let rows: Vec<Circle> = (1..=3).map(|id| ranked(id, Some(id as i32 + 10))).collect();
    assert_eq!(
        prefetch_ids(&rows, &pool, 3),
        vec![(1, 11), (2, 12), (3, 13)]
    );
    assert_eq!(
        prefetch_ids(&rows, &pool, 10),
        vec![(1, 11), (2, 12), (3, 13)]
    );
}

#[test]
fn prefetch_takes_the_best_ranks_first() {
    let pool = ClubPool::new((1..=10).map(|id| listed(id, 100.0 * id as f64)));
    let rows: Vec<Circle> = (1..=10)
        .map(|id| ranked(id, Some(100 - id as i32)))
        .collect();
    assert_eq!(
        prefetch_ids(&rows, &pool, 3),
        vec![(10, 90), (9, 91), (8, 92)]
    );
    assert!(prefetch_ids(&rows, &pool, 0).is_empty());
}

#[test]
fn prefetch_skips_clubs_outside_the_pool_and_repeats() {
    let pool = ClubPool::new([listed(1, 100.0), listed(2, 200.0), listed(3, 300.0)]);
    let rows = vec![
        ranked(4, Some(1)),
        ranked(2, Some(5)),
        ranked(2, Some(6)),
        ranked(3, None),
        ranked(1, Some(9)),
    ];
    assert_eq!(
        prefetch_ids(&rows, &pool, 10),
        vec![(2, 5), (1, 9), (3, u32::MAX)]
    );
}

#[test]
fn prefetch_skips_full_clubs() {
    let full = ListedClub {
        member_count: CLUB_CAP,
        ..listed(2, 200.0)
    };
    let pool = ClubPool::new([listed(1, 100.0), full]);
    let rows = vec![ranked(2, Some(1)), ranked(1, Some(2))];
    assert_eq!(prefetch_ids(&rows, &pool, 10), vec![(1, 2)]);
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

#[test]
fn shared_failures_keep_their_public_message() {
    let error = ScanError::Shared("listing clubs failed: could not reach uma.moe".into());
    assert_eq!(
        error.public_message(),
        "listing clubs failed: could not reach uma.moe"
    );
    assert_eq!(
        error.to_string(),
        "listing clubs failed: could not reach uma.moe"
    );
}

#[test]
fn the_page_cap_follows_the_rank_window() {
    let config = |rank_to, page_size| ScanConfig {
        rank_from: 1,
        rank_to,
        max_details: 10,
        page_size,
    };
    assert_eq!(max_pages(&config(900, 100)), 10);
    assert_eq!(max_pages(&config(10_000, 100)), 101);
    assert_eq!(max_pages(&config(64_036, 100)), 642);
    assert_eq!(max_pages(&config(5, 0)), 6);
}
