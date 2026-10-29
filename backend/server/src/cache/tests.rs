use super::*;
use std::time::Duration;
use uma_stable_match_umamoe_client::Error;

fn scan(ids: &[u64]) -> Scan {
    Scan {
        candidates: ids.len() + 1,
        prefetched: ids.len(),
        failed: 1,
        month: Some("2026-03".into()),
        updated_at: None,
    }
}

fn failure() -> ScanError {
    ScanError::List(Error::InvalidRequest("made-up failure".into()).into())
}

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn starts_empty() {
    let cache = Cache::new();
    assert_eq!(cache.snapshot(), None);
    assert_eq!(
        cache.status(),
        CacheStatus {
            scanning: false,
            snapshot: None,
            last_error: None,
            progress: None,
        }
    );
}

#[test]
fn only_one_scan_at_a_time() {
    let cache = Cache::new();
    assert!(cache.begin_scan());
    assert!(!cache.begin_scan());
    assert!(cache.status().scanning);
    cache.finish_scan(Ok(scan(&[1])), at(10));
    assert!(!cache.status().scanning);
    assert!(cache.begin_scan());
}

#[test]
fn success_replaces_snapshot_and_clears_error() {
    let cache = Cache::new();
    cache.begin_scan();
    cache.finish_scan(Err(failure()), at(10));
    cache.begin_scan();
    cache.finish_scan(Ok(scan(&[1, 2])), at(20));

    let snapshot = cache.snapshot().unwrap();
    assert_eq!(snapshot.prefetched, 2);
    assert_eq!(snapshot.candidates, 3);
    assert_eq!(snapshot.failed, 1);
    assert_eq!(snapshot.month.as_deref(), Some("2026-03"));
    assert_eq!(snapshot.scanned_at, at(20));
    assert_eq!(cache.status().last_error, None);
}

#[test]
fn failure_keeps_last_snapshot() {
    let cache = Cache::new();
    cache.begin_scan();
    cache.finish_scan(Ok(scan(&[1])), at(10));
    cache.begin_scan();
    cache.finish_scan(Err(failure()), at(20));

    let status = cache.status();
    assert!(!status.scanning);
    assert_eq!(status.snapshot.unwrap().scanned_at, at(10));
    let error = status.last_error.unwrap();
    assert_eq!(error.at, at(20));
    assert_eq!(error.message, "listing clubs failed: internal error");
}

#[test]
fn cancel_ends_the_scan_without_changes() {
    let cache = Cache::new();
    cache.begin_scan();
    cache.cancel_scan();
    assert_eq!(cache.status(), Cache::new().status());
}

#[test]
fn clones_share_state() {
    let cache = Cache::new();
    let clone = cache.clone();
    clone.begin_scan();
    clone.finish_scan(Ok(scan(&[7])), at(5));
    assert_eq!(cache.snapshot().unwrap().prefetched, 1);
}

#[test]
fn progress_is_reported_while_scanning_and_cleared_after() {
    let cache = Cache::new();
    cache.update_progress(|p| p.loaded = 5);
    assert_eq!(cache.status().progress, None);

    cache.begin_scan();
    cache.start_progress(ScanPhase::Listing, Some(at(100)));
    cache.update_progress(|p| {
        p.phase = ScanPhase::Loading;
        p.listed = 3;
        p.to_load = 2;
        p.loaded = 1;
    });
    assert_eq!(
        cache.status().progress,
        Some(ScanProgress {
            phase: ScanPhase::Loading,
            started_at: Some(at(100)),
            listed: 3,
            to_load: 2,
            loaded: 1,
            failed: 0,
        })
    );

    cache.finish_scan(Ok(scan(&[1])), at(200));
    assert_eq!(cache.status().progress, None);

    cache.begin_scan();
    cache.start_progress(ScanPhase::Waiting, None);
    cache.cancel_scan();
    assert_eq!(cache.status().progress, None);
}
